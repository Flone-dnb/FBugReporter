package main

import (
	"bufio"
	"fmt"
	"io"
	"io/fs"
	"io/ioutil"
	"os"
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

	if !is_llvm_installed(session) {
		fmt.Println("LLVM does not seem to be installed on this system.")
		fmt.Println("Please, install LLVM using this link: https://github.com/llvm/llvm-project/releases")
		fmt.Println("When installing LLVM pick \"Add LLVM to the system PATH for all users\".")
		fmt.Println("If you installed LLVM just now, maybe a reboot will help.")
		return
	}

	var ok = false
	var project_dir string
	var binary_dir_win string
	var binary_dir_linux string
	var lib_dir string
	var script_dir string

	for {
		fmt.Println()

		project_dir, ok = ask_directory("Specify your Godot game project directory (where your project's *.godot file is located):").Get()
		if !ok {
			return
		}

		var has_project = false
		has_project, ok = contains_godot_project(project_dir).Get()
		if !ok {
			return
		}

		if !has_project {
			fmt.Println("The directory", project_dir, "does not have a *.godot project file.")
			continue
		}

		break
	}

	fmt.Println()
	fmt.Println("We will need to ask you about 3 paths in order to install reporter, these are:")
	fmt.Println("- binary directory: this is a directory where we will put " +
		"compiled reporter program (the stuff that executes reporter functionality)")
	fmt.Println("- library directory: this is a directory where we will put GDNativeLibrary " +
		"files (Godot needs these files to know where reporter binaries are located)")
	fmt.Println("- script directory: this is a directory where we will put reporter GDScript file " +
		"(you will use functions from this script to send reports)")
	fmt.Println()

	var message string
	if runtime.GOOS == "windows" {
		message = "Specify where we should put the reporter library (reporter.dll) " +
			"(this should be located somewhere in your project directory, for example: " +
			"*your project directory*/bin/win64/):"
		binary_dir_win, ok = ask_directory(message).Get()
		if !ok {
			return
		}

		message = "Now specify the same thing but for Linux libraries (.so files), " +
			"for example: *your project directory*/bin/x11_64/:"
		binary_dir_linux, ok = ask_directory(message).Get()
		if !ok {
			return
		}
	} else {
		message = "Specify where we should put the reporter library (libreporter.so) " +
			"(this should be located somewhere in your project directory, for example: " +
			"*your project directory*/bin/x11_64/):"
		binary_dir_linux, ok = ask_directory(message).Get()
		if !ok {
			return
		}

		message = "Now specify the same thing but for Windows libraries (.dll files), " +
			"for example: *your project directory*/bin/win64/:"
		binary_dir_win, ok = ask_directory(message).Get()
		if !ok {
			return
		}
	}

	fmt.Println()

	lib_dir, ok = ask_directory("Specify where we should put reporter's GDNativeLibrary " +
		"files (this should be located somewhere in your project directory, for example: " +
		"*your project directory*/lib/fbugreporter/):").Get()
	if !ok {
		return
	}

	fmt.Println()

	script_dir, ok = ask_directory("Specify where we should put reporter's " +
		"GDScript file (the file that has function to send report) and reporter's scene file " +
		"(the file that has premade UI to send reports) (this should be located somewhere " +
		"in your project directory):").Get()
	if !ok {
		return
	}

	var binary_file_win = filepath.Join(binary_dir_win, "reporter.dll")
	var binary_file_linux = filepath.Join(binary_dir_linux, "libreporter.so")
	install_reporter(binary_file_win, binary_file_linux, project_dir, lib_dir, script_dir, session)
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

func is_llvm_installed(session *sh.Session) bool {
	fmt.Println("Checking that LLVM is installed on this system...")
	var err = session.Command("clang", "--version").Run()
	if err != nil {
		fmt.Println(err)
		return false
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

func install_reporter(binary_dst_win string, binary_dst_linux string, project_root_dir string, lib_dir string, script_dir string, session *sh.Session) {
	var wd, err = os.Getwd()
	if err != nil {
		fmt.Println(err)
		return
	}

	session.SetDir(filepath.Join(wd, "../../reporter"))

	// Check that Cargo.toml exists here.
	_, err = os.Stat(filepath.Join(session.Getwd(), "Cargo.toml"))
	if err == os.ErrNotExist {
		fmt.Println("Could not find reporter source code and 'Cargo.toml' file at", session.Getwd())
		return
	}

	fmt.Println("Found reporter source code at", session.Getwd())
	fmt.Println("Starting to compile reporter source code...")

	err = session.Command("cargo", "build", "--release").Run()
	if err != nil {
		fmt.Println(err)
		return
	}

	var binary_src string
	if runtime.GOOS == "windows" {
		binary_src = filepath.Join(session.Getwd(), "target", "release", "reporter.dll")
		fmt.Println("Adding reporter.dll")
		if copy(binary_src, binary_dst_win) {
			return
		}
	} else {
		binary_src = filepath.Join(session.Getwd(), "target", "release", "libreporter.so")
		fmt.Println("Adding libreporter.so")
		if copy(binary_src, binary_dst_linux) {
			return
		}
	}

	var gdns_name = "reporter.gdns"

	if write_lib_files(project_root_dir, lib_dir, binary_dst_win, binary_dst_linux, gdns_name) {
		return
	}

	if write_script_files(project_root_dir, lib_dir, script_dir, gdns_name) {
		return
	}

	fmt.Println()
	fmt.Println("Installation is finished.")
	fmt.Println("Now, edit the file", filepath.Join(script_dir, "reporter.gd"),
		"and change the line \"reporter.set_server(127, 0, 0, 1, 50123)\" in \"_ready()\" "+
			"according to your server's IP/port.")
}

// Returns empty if an error occurred.
func contains_godot_project(directory string) optional.Optional[bool] {
	var items, err = ioutil.ReadDir(directory)
	if err != nil {
		fmt.Println(err)
		return optional.Empty[bool]()
	}

	var found = false

	for _, item := range items {
		if !item.IsDir() {
			if strings.HasSuffix(item.Name(), ".godot") {
				found = true
			}
		}
	}

	return optional.Of(found)
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
func write_lib_files(project_root_dir string, lib_dir string, bin_file_win string, bin_file_linux string, gdns_name string) bool {
	var gdnlib_name = "reporter.gdnlib"

	fmt.Println("Adding reporter.gdnlib")

	// Just write, don't ask.
	var result = write_gdnlib(project_root_dir, lib_dir, bin_file_win, bin_file_linux, gdnlib_name)
	if result {
		return true
	}

	fmt.Println("Adding reporter.gdns")

	// Just write, don't ask.
	if runtime.GOOS == "windows" {
		return write_gdns(project_root_dir, lib_dir, bin_file_win, gdnlib_name, gdns_name)
	} else {
		return write_gdns(project_root_dir, lib_dir, bin_file_linux, gdnlib_name, gdns_name)
	}
}

func write_gdnlib(project_root_dir string, lib_dir string, bin_file_win string, bin_file_linux string, gdnlib_name string) bool {
	var cfg_file = filepath.Join(lib_dir, gdnlib_name)

	var cfg *ini.File
	var err error

	_, err = os.Stat(cfg_file)
	if os.IsNotExist(err) {
		cfg = ini.Empty()
	} else {
		fmt.Println(gdnlib_name, "already exists, skipping...")
		// Exists. Attempting to read will result in parsing error,
		// because 'dependencies' section contains arrays.
		return false
	}

	var bin_relative_win = strings.TrimPrefix(bin_file_win, project_root_dir)
	bin_relative_win = strings.TrimPrefix(bin_relative_win, "\\")
	bin_relative_win = strings.TrimPrefix(bin_relative_win, "/")
	bin_relative_win = strings.ReplaceAll(bin_relative_win, "\\", "/")

	var bin_relative_linux = strings.TrimPrefix(bin_file_linux, project_root_dir)
	bin_relative_linux = strings.TrimPrefix(bin_relative_linux, "\\")
	bin_relative_linux = strings.TrimPrefix(bin_relative_linux, "/")
	bin_relative_linux = strings.ReplaceAll(bin_relative_linux, "\\", "/")

	// -----------------------------------

	var section *ini.Section
	section, err = cfg.NewSection("general")
	if err != nil {
		fmt.Println(err)
		return true
	}

	_, err = section.NewKey("singleton", "false")
	if err != nil {
		fmt.Println(err)
		return true
	}
	_, err = section.NewKey("load_once", "true")
	if err != nil {
		fmt.Println(err)
		return true
	}

	if !section.HasKey("symbol_prefix") {
		_, err = section.NewKey("symbol_prefix", "\"godot_\"")
		if err != nil {
			fmt.Println(err)
			return true
		}
	}

	_, err = section.NewKey("reloadable", "true")
	if err != nil {
		fmt.Println(err)
		return true
	}

	// -----------------------------------

	section, err = cfg.NewSection("entry")
	if err != nil {
		fmt.Println(err)
		return true
	}

	_, err = section.NewKey("Windows.64", "\"res://"+bin_relative_win+"\"")
	if err != nil {
		fmt.Println(err)
		return true
	}
	_, err = section.NewKey("X11.64", "\"res://"+bin_relative_linux+"\"")
	if err != nil {
		fmt.Println(err)
		return true
	}

	// -----------------------------------

	section, err = cfg.NewSection("dependencies")
	if err != nil {
		fmt.Println(err)
		return true
	}

	_, err = section.NewKey("Windows.64", "[  ]")
	if err != nil {
		fmt.Println(err)
		return true
	}
	_, err = section.NewKey("X11.64", "[  ]")
	if err != nil {
		fmt.Println(err)
		return true
	}

	// -----------------------------------

	err = cfg.SaveTo(cfg_file)
	if err != nil {
		fmt.Println(err)
		return true
	}

	return false
}

func write_gdns(project_root_dir string, lib_dir string, bin_file string, gdnlib_name string, gdns_name string) bool {
	var cfg_file = filepath.Join(lib_dir, gdns_name)
	var cfg *ini.File

	var _, err = os.Stat(cfg_file)
	if os.IsNotExist(err) {
		cfg = ini.Empty()
	} else {
		fmt.Println(gdns_name, "already exists, skipping...")
		return false
	}

	var gdnlib_relative = strings.TrimPrefix(lib_dir, project_root_dir)
	if runtime.GOOS == "windows" {
		gdnlib_relative = strings.TrimPrefix(gdnlib_relative, "\\")
	} else {
		gdnlib_relative = strings.TrimPrefix(gdnlib_relative, "/")
	}

	gdnlib_relative = filepath.Join(gdnlib_relative, gdnlib_name)
	gdnlib_relative = strings.ReplaceAll(gdnlib_relative, "\\", "/")

	// -----------------------------------

	var section *ini.Section
	_, err = cfg.NewSection("gd_resource type=\"NativeScript\" load_steps=2 format=2")
	if err != nil {
		fmt.Println(err)
		return true
	}

	_, err = cfg.NewSection("ext_resource path=\"res://" + gdnlib_relative + "\" type=\"GDNativeLibrary\" id=1")
	if err != nil {
		fmt.Println(err)
		return true
	}

	// -----------------------------------

	section, err = cfg.NewSection("resource")
	if err != nil {
		fmt.Println(err)
		return true
	}

	_, err = section.NewKey("resource_name", "\"Reporter\"")
	if err != nil {
		fmt.Println(err)
		return true
	}

	_, err = section.NewKey("class_name", "\"Reporter\"")
	if err != nil {
		fmt.Println(err)
		return true
	}

	_, err = section.NewKey("library", "ExtResource( 1 )")
	if err != nil {
		fmt.Println(err)
		return true
	}

	// -----------------------------------

	err = cfg.SaveTo(cfg_file)
	if err != nil {
		fmt.Println(err)
		return true
	}

	return false
}

// Returns 'true' if an error occurred.
func write_script_files(project_root_dir string, lib_dir string, script_dir string, gdns_name string) bool {
	var gdns_relative = strings.TrimPrefix(lib_dir, project_root_dir)
	if runtime.GOOS == "windows" {
		gdns_relative = strings.TrimPrefix(gdns_relative, "\\")
	} else {
		gdns_relative = strings.TrimPrefix(gdns_relative, "/")
	}

	gdns_relative = filepath.Join(gdns_relative, gdns_name)
	gdns_relative = strings.ReplaceAll(gdns_relative, "\\", "/")

	var wd, err = os.Getwd()
	if err != nil {
		fmt.Println(err)
		return true
	}

	var gd_name = "reporter.gd"

	fmt.Println("Adding", gd_name)

	var gd_src = filepath.Join(wd, "../../example/MainScene.gd")
	var gd_dst = filepath.Join(script_dir, gd_name)

	_, err = os.Stat(gd_src)
	if err == os.ErrNotExist {
		fmt.Println("Could not find ", gd_src)
		return true
	}

	_, err = os.Stat(gd_dst)
	if err == nil {
		// Already exists.
		var yes, ok = ask_user(fmt.Sprint("The file ", gd_dst,
			" already exists, do you want to overwrite it? (y/n)")).Get()
		if !ok {
			return true
		}

		if yes {
			copy(gd_src, gd_dst)
		}
	} else {
		copy(gd_src, gd_dst)
	}

	if replace_string_in_file(gd_dst, "lib/reporter.gdns", gdns_relative) {
		return true
	}

	fmt.Println("Adding reporter.tscn")

	var tscn_src = filepath.Join(wd, "../../example/MainScene.tscn")
	var tscn_dst = filepath.Join(script_dir, "reporter.tscn")

	_, err = os.Stat(tscn_src)
	if err == os.ErrNotExist {
		fmt.Println("Could not find ", tscn_src)
		return true
	}

	_, err = os.Stat(tscn_dst)
	if err == nil {
		var yes, ok = ask_user(fmt.Sprint("The file ", tscn_dst,
			" already exists, do you want to overwrite it? (y/n)")).Get()
		if !ok {
			return true
		}

		if yes {
			copy(tscn_src, tscn_dst)
		}
	} else {
		copy(tscn_src, tscn_dst)
	}

	var script_relative = strings.TrimPrefix(script_dir, project_root_dir)
	if runtime.GOOS == "windows" {
		script_relative = strings.TrimPrefix(script_relative, "\\")
	} else {
		script_relative = strings.TrimPrefix(script_relative, "/")
	}

	script_relative = filepath.Join(script_relative, gd_name)
	script_relative = strings.ReplaceAll(script_relative, "\\", "/")

	return replace_string_in_file(tscn_dst, "MainScene.gd", script_relative)
}

// Returns 'true' if an error occurred.
func replace_string_in_file(file_path string, replace_from string, replace_to string) bool {
	var err error
	var readFile *os.File
	readFile, err = os.Open(file_path)
	if err != nil {
		fmt.Println(err)
		return true
	}

	var write_file *os.File
	write_file, err = os.Create(file_path + "~")
	if err != nil {
		fmt.Println(err)
		return true
	}

	var fileScanner = bufio.NewScanner(readFile)
	fileScanner.Split(bufio.ScanLines)

	var writer = bufio.NewWriter(write_file)

	var found bool = false
	for fileScanner.Scan() {
		var line = fileScanner.Text()

		if strings.Contains(line, replace_from) {
			found = true
			line = strings.ReplaceAll(line, replace_from, replace_to)
		}

		writer.WriteString(line + "\n")
	}

	readFile.Close()
	writer.Flush()
	write_file.Close()

	err = os.Remove(file_path)
	if err != nil {
		fmt.Println(err)
		return true
	}

	err = os.Rename(file_path+"~", file_path)
	if err != nil {
		fmt.Println(err)
		return true
	}

	if !found {
		fmt.Println("Failed to find replace from string", replace_from, "to replace.")
		return true
	}

	return false
}
