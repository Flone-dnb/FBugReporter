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

// Name of the GDExtension file that reporter uses.
var gdext_name = "FBugReporter.gdextension"

// Name of the file that Godot 4 uses for active extension list.
var extension_list_name = "extension_list.cfg"

// Name of the GD script file used for sending reports.
var reporter_script_name = "reporter.gd"

// Name of the Godot scene file with a premade UI for sending reports.
var reporter_scene_name = "reporter.tscn"

// Text inside of the reporter's scene file that references reporter's script file.
var reporter_scene_script_relative_path = "res://scenes/reporter.gd"

// Godot 4 directory that stores internal project files.
var dotgodot_dir_name = ".godot"

// Relative path to the directory where example Godot project is located.
var relative_path_to_example_dir = "../../example/"

// Path relative to example project's root directory that stores reporter's script and scene files.
var relative_project_path_to_script_files = "scenes"

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
	var gdext_dir string
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
	fmt.Println("- GDExtension directory: this is a directory where we will put GDExtension " +
		"file (Godot needs these files to know where reporter binaries are located)")
	fmt.Println("- script directory: this is a directory where we will put reporter GDScript files " +
		"(you will use functions from this script to send reports)")
	fmt.Println()

	var message string
	if runtime.GOOS == "windows" {
		message = "Specify where we should put the reporter library (reporter.dll) " +
			"(this should be located somewhere in your project directory, for example: " +
			"*your project directory*/bin/windows/):"
		binary_dir_win, ok = ask_directory(message).Get()
		if !ok {
			return
		}

		fmt.Println()

		message = "Now specify the same thing but for Linux libraries (.so files), " +
			"for example: *your project directory*/bin/linux/:"
		binary_dir_linux, ok = ask_directory(message).Get()
		if !ok {
			return
		}
	} else {
		message = "Specify where we should put the reporter library (libreporter.so) " +
			"(this should be located somewhere in your project directory, for example: " +
			"*your project directory*/bin/linux/):"
		binary_dir_linux, ok = ask_directory(message).Get()
		if !ok {
			return
		}

		fmt.Println()

		message = "Now specify the same thing but for Windows libraries (.dll files), " +
			"for example: *your project directory*/bin/windows/:"
		binary_dir_win, ok = ask_directory(message).Get()
		if !ok {
			return
		}
	}

	fmt.Println()

	gdext_dir, ok = ask_directory("Specify where we should put reporter's GDExtension " +
		"file (this should be located somewhere in your project directory, for example: " +
		"*your project directory*/extensions/):").Get()
	if !ok {
		return
	}

	fmt.Println()

	script_dir, ok = ask_directory("Specify where we should put reporter's " +
		"GDScript file (the file that has a function to send report) and reporter's scene file " +
		"(the file that has premade UI to send reports) (this should be located somewhere " +
		"in your project directory):").Get()
	if !ok {
		return
	}

	var binary_file_win = filepath.Join(binary_dir_win, "reporter.dll")
	var binary_file_linux = filepath.Join(binary_dir_linux, "libreporter.so")
	install_reporter(binary_file_win, binary_file_linux, project_dir, gdext_dir, script_dir, session)
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

func install_reporter(binary_dst_win string, binary_dst_linux string, project_root_dir string, gdext_dir string, script_dir string, session *sh.Session) {
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

	// Write GDExtension file.
	if write_gdext(project_root_dir, gdext_dir, binary_dst_win, binary_dst_linux) {
		return
	}

	// Write script files.
	if write_script_files(project_root_dir, script_dir) {
		return
	}

	fmt.Println()
	fmt.Println("Installation is finished.")
	fmt.Println("Now, edit the file", filepath.Join(script_dir, reporter_script_name),
		"and change the line \"reporter.setup_report_receiver(\"Server\", \"127.0.0.1:50123\", \"\")\" in \"func _ready()\" "+
			"according to your server's IP/port.\n"+
			"It's also highly recommended to look at", reporter_script_name, "file and understand how it works.")
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

// Creates a new reporter's GDExtension file (removes old one if exists) at the specified GDExtension path.
// Returns `true` if an error occurs.
func write_gdext(project_root_dir string, gdext_dir string, bin_file_win string, bin_file_linux string) bool {
	var gdext_file_path = filepath.Join(gdext_dir, gdext_name)

	// Removing existing gdextension file if exists (may contain outdated information).
	var _, err = os.Stat(gdext_file_path)
	if err == nil {
		os.Remove(gdext_file_path)
	}

	// Make paths to be relative to project root directory and replace any "\\" with "/".
	var bin_relative_win = strings.TrimPrefix(bin_file_win, project_root_dir)
	bin_relative_win = strings.TrimPrefix(bin_relative_win, "\\")
	bin_relative_win = strings.TrimPrefix(bin_relative_win, "/")
	bin_relative_win = strings.ReplaceAll(bin_relative_win, "\\", "/")

	var bin_relative_linux = strings.TrimPrefix(bin_file_linux, project_root_dir)
	bin_relative_linux = strings.TrimPrefix(bin_relative_linux, "\\")
	bin_relative_linux = strings.TrimPrefix(bin_relative_linux, "/")
	bin_relative_linux = strings.ReplaceAll(bin_relative_linux, "\\", "/")

	// ---------------------------------------------------------------------

	// Fill ini file.
	var cfg = ini.Empty()

	var section *ini.Section
	// create a raw section to write WITH " symbols because `NewKey` does not add them here,
	// the current Godot 4 version fails to parse this line without them
	_, err = cfg.NewRawSection("configuration", "entry_symbol = \"gdext_rust_init\"")
	if err != nil {
		fmt.Println(err)
		return true
	}

	section, err = cfg.NewSection("libraries")
	if err != nil {
		fmt.Println(err)
		return true
	}

	_, err = section.NewKey("windows.debug.x86_64", "\"res://"+bin_relative_win+"\"")
	if err != nil {
		fmt.Println(err)
		return true
	}
	_, err = section.NewKey("windows.release.x86_64", "\"res://"+bin_relative_win+"\"")
	if err != nil {
		fmt.Println(err)
		return true
	}
	_, err = section.NewKey("linux.debug.x86_64", "\"res://"+bin_relative_linux+"\"")
	if err != nil {
		fmt.Println(err)
		return true
	}
	_, err = section.NewKey("linux.release.x86_64", "\"res://"+bin_relative_linux+"\"")
	if err != nil {
		fmt.Println(err)
		return true
	}

	// ---------------------------------------------------------------------

	// Save filled ini file.
	err = cfg.SaveTo(gdext_file_path)
	if err != nil {
		fmt.Println(err)
		return true
	}

	// Update GDExtension list.

	// Prepare a line that we will have in the extension list.

	// Make path to be relative to project root directory and replace any "\\" with "/".
	var gdext_relative = strings.TrimPrefix(gdext_file_path, project_root_dir)
	gdext_relative = strings.TrimPrefix(gdext_relative, "\\")
	gdext_relative = strings.TrimPrefix(gdext_relative, "/")
	gdext_relative = strings.ReplaceAll(gdext_relative, "\\", "/")

	// Construct a path to the extension list file.
	var path_to_ext_list = filepath.Join(project_root_dir, dotgodot_dir_name, extension_list_name)
	_, err = os.Stat(path_to_ext_list)
	if os.IsNotExist(err) {
		// Create a new extension list file.
		_, err = os.Create(path_to_ext_list)
		if err != nil {
			fmt.Println(err)
			return true
		}
	} else {
		// Check if extension is already enabled.
		if does_extension_list_contains_reporter(path_to_ext_list, gdext_relative) {
			// Nothing to do.
			return false
		}
	}

	// Add extension line.
	f, err := os.OpenFile(path_to_ext_list, os.O_APPEND|os.O_WRONLY, 0644)
	if err != nil {
		fmt.Println(err)
		return true
	}
	defer f.Close()

	_, err = f.WriteString("\nres://" + gdext_relative + "\n")
	if err != nil {
		fmt.Println(err)
		return true
	}

	fmt.Println("Enabled", gdext_relative, "in the", extension_list_name)

	return false
}

func does_extension_list_contains_reporter(extension_list_file_path string, string_to_look_for string) bool {
	f, err := os.Open(extension_list_file_path)
	if err != nil {
		return false
	}
	defer f.Close()

	// Splits on newlines by default.
	scanner := bufio.NewScanner(f)

	line := 1
	for scanner.Scan() {
		if strings.Contains(scanner.Text(), string_to_look_for) {
			fmt.Println("Found already enabled", string_to_look_for, "in the", extension_list_name, "file")
			return true
		}

		line++
	}

	if err := scanner.Err(); err != nil {
		return false
	}

	return false
}

// Adds GDScript and scene files.
// Returns `true` if an error occurred.
func write_script_files(project_root_dir string, script_dir string) bool {
	// Prepare a string to store relative path to the script file.
	// We will write it to the copied scene file.
	var script_relative = strings.TrimPrefix(script_dir, project_root_dir)
	script_relative = strings.TrimPrefix(script_relative, "\\")
	script_relative = strings.TrimPrefix(script_relative, "/")
	script_relative = strings.ReplaceAll(script_relative, "\\", "/")
	script_relative = strings.TrimSuffix(script_relative, "/")
	script_relative += "/" + reporter_script_name
	script_relative = "res://" + script_relative

	// Get working directory.
	var wd, err = os.Getwd()
	if err != nil {
		fmt.Println(err)
		return true
	}

	fmt.Println("Adding", reporter_scene_name)

	// Prepare paths to script files (to/from).
	var src_path = filepath.Join(wd, relative_path_to_example_dir, relative_project_path_to_script_files, reporter_scene_name)
	var dst_path = filepath.Join(script_dir, reporter_scene_name)

	// Make sure example scene file exists.
	_, err = os.Stat(src_path)
	if err == os.ErrNotExist {
		fmt.Println("Expected to find a scene file at", src_path)
		return true
	}

	// Check if some file already exists at the destination path.
	_, err = os.Stat(dst_path)
	var should_copy = true
	if err == nil {
		// Already exists.
		var yes, ok = ask_user(fmt.Sprint("The file ", dst_path,
			" already exists, do you want to overwrite it? (y/n)")).Get()
		if !ok {
			return true
		}

		if !yes {
			should_copy = false
		}
	}
	if should_copy {
		if copy(src_path, dst_path) {
			return true
		}
	}

	// Replace path to the script file.
	if replace_string_in_file(dst_path, reporter_scene_script_relative_path, script_relative) {
		return true
	}

	fmt.Println("Adding", reporter_script_name)

	src_path = filepath.Join(wd, relative_path_to_example_dir, relative_project_path_to_script_files, reporter_script_name)
	dst_path = filepath.Join(script_dir, reporter_script_name)

	// Make sure example script file exists.
	_, err = os.Stat(src_path)
	if err == os.ErrNotExist {
		fmt.Println("Expected to find a script file at", src_path)
		return true
	}

	_, err = os.Stat(dst_path)
	should_copy = true
	if err == nil {
		var yes, ok = ask_user(fmt.Sprint("The file ", dst_path,
			" already exists, do you want to overwrite it? (y/n)")).Get()
		if !ok {
			return true
		}

		if !yes {
			should_copy = false
		}
	}
	if should_copy {
		if copy(src_path, dst_path) {
			return true
		}
	}

	return false
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
