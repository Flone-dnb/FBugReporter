package main

import (
	"bufio"
	"fmt"
	"io"
	"io/fs"
	"os"
	"path/filepath"
	"runtime"
	"strings"

	"4d63.com/optional"
	"github.com/codeskyblue/go-sh"
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

	for {
		fmt.Println("Where do you want the client application to be installed? (insert path to the directory)")

		var reader = bufio.NewReader(os.Stdin)
		var client_install_path, err = reader.ReadString('\n')
		if err != nil {
			fmt.Println("An error occurred while reading input:", err)
			return
		}

		if runtime.GOOS == "windows" {
			client_install_path = strings.TrimSuffix(client_install_path, "\r\n")
		} else {
			client_install_path = strings.TrimSuffix(client_install_path, "\n")
		}

		// Check if this is a directory.
		var file_info fs.FileInfo
		file_info, err = os.Stat(client_install_path)
		if err != nil {
			fmt.Println("An error occurred while reading input:", err)
			return
		}
		if !file_info.IsDir() {
			fmt.Println("Please, specify a directory path.")
			continue
		}
		if file_info.Mode().Perm()&(1<<(uint(7))) == 0 {
			fmt.Println("This directory is not writable, please choose a different directory.")
			continue
		}

		// Check that this directory is empty.
		var empty bool
		empty, err = is_empty(client_install_path)
		if err != nil {
			fmt.Println(err)
		}
		if !empty {
			var yes, ok = ask_user("The specified directory is not empty, continue? (y/n)").Get()
			if !ok {
				return
			}
			if !yes {
				continue
			}
		}

		client_install_path = append_client_binary_to_path(client_install_path)

		fmt.Println("The client application will be installed in the following path:")
		fmt.Println(client_install_path)

		var install, ok = ask_user("Is this correct? (y/n)").Get()
		if !ok {
			return
		}

		if install {
			install_client(client_install_path, session)
			break
		}
	}
}

func is_rust_installed(session *sh.Session) bool {
	var err = session.Command("cargo", "--version").Run()
	if err != nil {
		fmt.Println(err)
		return false
	}

	return true
}

func append_client_binary_to_path(path string) string {
	if runtime.GOOS == "windows" {
		path = filepath.Join(path, "client.exe")
	} else {
		path = filepath.Join(path, "client")
	}

	return path
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

func install_client(resulting_binary_path string, session *sh.Session) {
	var wd, err = os.Getwd()
	if err != nil {
		fmt.Println(err)
		return
	}

	session.SetDir(filepath.Join(wd, "../../client"))

	// Check that Cargo.toml exists here.
	_, err = os.Stat(filepath.Join(session.Getwd(), "Cargo.toml"))
	if err == os.ErrNotExist {
		fmt.Println("Could not find client source code and 'Cargo.toml' file at", session.Getwd())
		return
	}

	fmt.Println("Found client source code at", session.Getwd())
	fmt.Println("Starting to compile client source code...")

	err = session.Command("cargo", "build", "--release").Run()
	if err != nil {
		fmt.Println(err)
		return
	}

	var binary string
	if runtime.GOOS == "windows" {
		binary = filepath.Join(session.Getwd(), "target", "release", "client.exe")
	} else {
		binary = filepath.Join(session.Getwd(), "target", "release", "client")
	}

	if copy(binary, resulting_binary_path) {
		return
	}

	fmt.Println("The client application was successfully compiled, you will find it at", resulting_binary_path)
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

func is_empty(name string) (bool, error) {
	f, err := os.Open(name)
	if err != nil {
		return false, err
	}
	defer f.Close()

	_, err = f.Readdirnames(1)
	if err == io.EOF {
		return true, nil
	}
	return false, err
}
