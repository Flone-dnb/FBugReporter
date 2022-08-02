package main

import (
	"bufio"
	"fmt"
	"io"
	"io/fs"
	"log"
	"os"
	"os/user"
	"path/filepath"
	"runtime"
	"strings"

	"4d63.com/optional"
	"github.com/codeskyblue/go-sh"
	"github.com/go-ini/ini"
)

func main() {
	var session = sh.NewSession()
	session.PipeFail = true
	session.PipeStdErrors = true

	if !is_rust_installed(session) {
		fmt.Println("Rust does not seem to be installed on this system.")
		fmt.Println("Please, install Rust using this link: https://www.rust-lang.org/tools/install")
		fmt.Println("If you installed Rust just now, maybe a reboot will help.")
		return
	}

	if !is_sqlite3_installed(session) {
		return
	}

	var ok = false
	var install_dir string

	fmt.Println()

	fmt.Println("The server consists of 3 programs:")
	fmt.Println("- server: the actual server")
	fmt.Println("- database manager: used to add/remove users (even when the server is running)")
	fmt.Println("- server_monitor: simple helper app that will restart the server if it crashed, " +
		"you don't need to explicitly start the 'server' program, instead, run 'server_monitor' " +
		"it will run the 'server'.")

	fmt.Println()
	install_dir, ok = ask_directory("Where do you want to install all these programs?").Get()
	if !ok {
		return
	}

	install_server(install_dir, session)
}

func is_rust_installed(session *sh.Session) bool {
	fmt.Println("Checking that Rust is installed on this system...")
	var err = session.Command("cargo", "--version").Run()
	if err != nil {
		fmt.Println(err)
		return false
	}

	return true
}

func is_sqlite3_installed(session *sh.Session) bool {
	if runtime.GOOS != "windows" {
		fmt.Println("Checking that SQLite3 is installed on this system...")
		var err = session.Command("sqlite3", "--version").Run()
		if err != nil {
			fmt.Println(err)
			fmt.Println()
			fmt.Println("SQLite3 does not seem to be installed on this system.")
			fmt.Println("Please, install SQLite3 from your package manager.")
			return false
		}
	}

	return true
}

// Returns empty if an error occurred.
func ask_directory(question string) optional.Optional[string] {
	for {
		fmt.Println(question)

		var reader = bufio.NewReader(os.Stdin)
		var directory_path, err = reader.ReadString('\n')
		if err != nil {
			fmt.Println("An error occurred while reading input:", err)
			return optional.Empty[string]()
		}

		if runtime.GOOS == "windows" {
			directory_path = strings.TrimSuffix(directory_path, "\r\n")
		} else {
			directory_path = strings.TrimSuffix(directory_path, "\n")
		}

		// Check if this is a directory.
		var file_info fs.FileInfo
		file_info, err = os.Stat(directory_path)
		if err != nil {
			fmt.Println("An error occurred while reading input:", err)
			return optional.Empty[string]()
		}
		if !file_info.IsDir() {
			fmt.Println("Please, specify a directory path.")
			continue
		}
		if file_info.Mode().Perm()&(1<<(uint(7))) == 0 {
			fmt.Println("This directory is not writable, please choose a different directory.")
			continue
		}

		fmt.Println("The following directory will be used:")
		fmt.Println(directory_path)
		var yes, ok = ask_user("Are you sure you want to use this directory? (y/n)").Get()
		if !ok {
			return optional.Empty[string]()
		}

		if yes {
			return optional.Of(directory_path)
		}
	}
}

// Returns empty if an error occurred.
// Returns 'true' if the user answered 'yes'.
// Returns 'false' if the user answered 'no'.
func ask_user(question string) optional.Optional[bool] {
	for {
		fmt.Println(question)

		var reader = bufio.NewReader(os.Stdin)
		var text, err = reader.ReadString('\n')
		if err != nil {
			fmt.Println("An error occurred:", err)
			return optional.Empty[bool]()
		}

		if runtime.GOOS == "windows" {
			text = strings.TrimSuffix(text, "\r\n")
		} else {
			text = strings.TrimSuffix(text, "\n")
		}

		text = strings.ToLower(text)
		if text == "y" || text == "yes" {
			return optional.Of(true)
		}

		if text == "n" || text == "no" {
			return optional.Of(false)
		}

		fmt.Println(text, "is not a valid input. Please, provide a valid input.")
	}
}

func install_server(install_dir string, session *sh.Session) {
	var wd, err = os.Getwd()
	if err != nil {
		fmt.Println(err)
		return
	}

	session.SetDir(filepath.Join(wd, "../../server"))

	if install_server_app(install_dir, session) {
		return
	}

	if install_database_manager_app(install_dir, session) {
		return
	}

	if install_server_monitor_app(install_dir, session) {
		return
	}

	fmt.Println()
	fmt.Println("Installation is finished.")
	fmt.Println("Note that you should not run the 'server' explicitly, instead, " +
		"run the 'server_monitor' it will run the 'server'.")
	fmt.Println()

	if runtime.GOOS != "windows" {
		err = session.Command("chmod", "+x", filepath.Join(install_dir, "server")).Run()
		if err != nil {
			fmt.Println(err)
			return
		}

		err = session.Command("chmod", "+x", filepath.Join(install_dir, "server_monitor")).Run()
		if err != nil {
			fmt.Println(err)
			return
		}

		err = session.Command("chmod", "+x", filepath.Join(install_dir, "database_manager")).Run()
		if err != nil {
			fmt.Println(err)
			return
		}

		var yes, ok = ask_user("Do you want to install systemd service to autostart the " +
			"'server_monitor'? (y/n)").Get()
		if !ok {
			return
		}

		if yes {
			if install_systemd_service(install_dir, session) {
				return
			}
		}

		fmt.Println()
		fmt.Println("Finished installation.")
	} else {
		fmt.Println("Note that the 'server_monitor' is not added to autostart, it's up to you to do so.")
	}
}

// Expects to be in the root server directory that contains all 3 programs.
// Returns 'true' if an error occurred.
func install_server_app(install_dir string, session *sh.Session) bool {
	var root_dir = session.Getwd()

	session.SetDir(filepath.Join(root_dir, "server"))

	// Check that Cargo.toml exists here.
	var _, err = os.Stat(filepath.Join(session.Getwd(), "Cargo.toml"))
	if os.IsNotExist(err) {
		fmt.Println("Could not find server source code and 'Cargo.toml' file at", session.Getwd())
		return true
	}

	if runtime.GOOS == "windows" {
		// Add env variable to link sqlite3.
		session.SetEnv("SQLITE3_LIB_DIR", filepath.Join(root_dir, "sqlite3-windows"))
	}

	fmt.Println("Found server source code at", session.Getwd())
	fmt.Println("Starting to compile server source code...")

	err = session.Command("cargo", "build", "--release").Run()
	if err != nil {
		fmt.Println(err)
		return true
	}

	if runtime.GOOS == "windows" {
		copy(filepath.Join(session.Getwd(), "target", "release", "server.exe"), filepath.Join(install_dir, "server.exe"))
	} else {
		copy(filepath.Join(session.Getwd(), "target", "release", "server"), filepath.Join(install_dir, "server"))
	}

	if runtime.GOOS == "windows" {
		// Copy sqlite3.dll.
		copy(filepath.Join(root_dir, "sqlite3-windows", "sqlite3.dll"), filepath.Join(install_dir, "sqlite3.dll"))
	}

	session.SetDir(root_dir)

	return false
}

// Expects to be in the root server directory that contains all 3 programs
// Returns 'true' if an error occurred.
func install_database_manager_app(install_dir string, session *sh.Session) bool {
	var root_dir = session.Getwd()

	session.SetDir(filepath.Join(root_dir, "database_manager"))

	// Check that Cargo.toml exists here.
	var _, err = os.Stat(filepath.Join(session.Getwd(), "Cargo.toml"))
	if err == os.ErrNotExist {
		fmt.Println("Could not find database manager source code and 'Cargo.toml' file at", session.Getwd())
		return true
	}

	fmt.Println("Found database manager source code at", session.Getwd())
	fmt.Println("Starting to compile database manager source code...")

	err = session.Command("cargo", "build", "--release").Run()
	if err != nil {
		fmt.Println(err)
		return true
	}

	if runtime.GOOS == "windows" {
		copy(filepath.Join(session.Getwd(), "target", "release", "database_manager.exe"), filepath.Join(install_dir, "database_manager.exe"))
	} else {
		copy(filepath.Join(session.Getwd(), "target", "release", "database_manager"), filepath.Join(install_dir, "database_manager"))
	}

	session.SetDir(root_dir)

	return false
}

// Expects to be in the root server directory that contains all 3 programs
// Returns 'true' if an error occurred.
func install_server_monitor_app(install_dir string, session *sh.Session) bool {
	var root_dir = session.Getwd()

	session.SetDir(filepath.Join(root_dir, "server_monitor"))

	// Check that Cargo.toml exists here.
	var _, err = os.Stat(filepath.Join(session.Getwd(), "Cargo.toml"))
	if err == os.ErrNotExist {
		fmt.Println("Could not find server monitor source code and 'Cargo.toml' file at", session.Getwd())
		return true
	}

	fmt.Println("Found server monitor source code at", session.Getwd())
	fmt.Println("Starting to compile server monitor source code...")

	err = session.Command("cargo", "build", "--release").Run()
	if err != nil {
		fmt.Println(err)
		return true
	}

	if runtime.GOOS == "windows" {
		copy(filepath.Join(session.Getwd(), "target", "release", "server_monitor.exe"), filepath.Join(install_dir, "server_monitor.exe"))
	} else {
		copy(filepath.Join(session.Getwd(), "target", "release", "server_monitor"), filepath.Join(install_dir, "server_monitor"))
	}

	session.SetDir(root_dir)

	return false
}

// Returns 'true' if failed
func copy(src string, dst string) bool {
	sourceFileStat, err := os.Stat(src)
	if err != nil {
		fmt.Println(err)
		return true
	}

	if !sourceFileStat.Mode().IsRegular() {
		fmt.Println(src, "is not a file")
		return true
	}

	source, err := os.Open(src)
	if err != nil {
		fmt.Println(err)
		return true
	}
	defer source.Close()

	destination, err := os.Create(dst)
	if err != nil {
		fmt.Println(err)
		return true
	}
	defer destination.Close()
	_, err = io.Copy(destination, source)
	if err != nil {
		fmt.Println(err)
		return true
	}

	return false
}

// Returns 'true' if an error occurred.
func install_systemd_service(install_dir string, session *sh.Session) bool {
	var cfg = ini.Empty()

	var currentUser, err = user.Current()
	if err != nil {
		log.Fatalf(err.Error())
	}

	var group *user.Group
	group, err = user.LookupGroup(currentUser.Username)
	if err != nil {
		log.Fatalf(err.Error())
	}

	var section *ini.Section
	section, err = cfg.NewSection("Unit")
	if err != nil {
		fmt.Println(err)
		return true
	}

	section.NewKey("Description", "FBugReporter Server")

	section, err = cfg.NewSection("Service")
	if err != nil {
		fmt.Println(err)
		return true
	}

	section.NewKey("WorkingDirectory", install_dir)
	section.NewKey("ExecStart", filepath.Join(install_dir, "server_monitor"))
	section.NewKey("User", currentUser.Name)
	section.NewKey("Group", group.Name)

	section, err = cfg.NewSection("Install")
	if err != nil {
		fmt.Println(err)
		return true
	}

	section.NewKey("WantedBy", "multi-user.target")

	err = cfg.SaveTo(filepath.Join(install_dir, "fbugreporter.service"))
	if err != nil {
		fmt.Println(err)
		return true
	}

	err = session.Command("sudo", "mv", filepath.Join(install_dir, "fbugreporter.service"), "/etc/systemd/system").Run()
	if err != nil {
		fmt.Println(err)
		return true
	}

	err = session.Command("sudo", "systemctl", "enable", "fbugreporter.service").Run()
	if err != nil {
		fmt.Println(err)
		return true
	}

	err = session.Command("sudo", "systemctl", "start", "fbugreporter.service").Run()
	if err != nil {
		fmt.Println(err)
		return true
	}

	fmt.Println()
	fmt.Println("The 'server_monitor' was started and added as a service. " +
		"It will autostart on boot.")
	fmt.Println("Use \"systemctl status fbugreporter.service\" to view current status/logs.")
	fmt.Println()
	fmt.Println("Use \"sudo systemctl disable fbugreporter.service\" to disable autostart.")
	fmt.Println("And \"sudo rm /etc/systemd/system/fbugreporter.service\" to remove it.")

	return false
}
