# FBugReporter

![](screenshot.png?raw=true)

# Supported Platforms

- Windows
- Linux

# Try It Out

Windows users can find built versions of FBugReporter in the [releases](https://github.com/Flone-dnb/FBugReporter/releases) section (with `*_Build.zip` suffix). This archive contains an example Godot project with FBugReporter already integrated, using this example you can send reports. In order to receive reports this archive also has `monitor.exe` (that will start the `server.exe`) that you need to start in order to receive reports. To view received reports you can use `client.exe` but it requires an account which you can create after starting the `monitor.exe` using the `database_manager.exe` (type `add-user *your name*` to create an account).

The usual workflow goes like this:

- Start the `monitor.exe`, it will start the `server.exe` and will restart it if it crashed (thus we should start `monitor.exe` instead of explicitly starting the `server.exe`).
- After `monitor.exe` was started, run `database_manager.exe` and type command `add-user *your name*` to add a new user (for example: `add-user john`), you will receive new user's password, copy it somewhere.
- Run `client.exe` (`monitor.exe` needs to be running), in order to login you need to enter server's IP and port. For local usage put `localhost` as IP. For port look for `server_config.ini` (that will be generated once the `monitor.exe` is started), look at `port_for_clients` line that will contain the port you need to use. Now login using the specified earlier name and the password you received, after this you will change the password and setup OTP. After everything is done you will see received reports.
- To generate new reports, open Godot project with FBugReporter integrated (build version from releases already has a project with FBugReporter integrated) and send a report (while `monitor.exe` is running). You can then see new reports in `client.exe` (restart `client.exe` in order to refresh report list, later when you will have some reports the report list will be refreshed after you return from viewing some report to the main page or when you switch pages).

# Install

To make the process of installation easier, I wrote a few script that you can find in the `install` directory of this repository.

In order to run these scripts you need to have [Go](https://go.dev/dl/) installed. To run a script open a directory with the `*.go` file, and type `cmd` in the explorer's address bar, once the console is opened, type `go run .` in order to start the script.

- `install_client.go` this script will ask you where you want the client to be installed and will compile the client. It will also check if needed dependencies are installed and if not will prompt you to install them.
- `install_reporter.go` this script will ask you where your Godot project is located and will compile and integrate reporter into your project (with premade UI scene to send reports). It will also check if needed dependencies are installed and if not will prompt you to install them.
- `install_server.go` this script will ask you where you want the server to be installed and will compile the server, monitor and database manager for you. It will also check if needed dependencies are installed and if not will prompt you to install them.

If you want to integrate FBugReporter into your Godot game, just clone/download this repository and run each script (the order does not matter), they will setup everything for you.

If you want to update FBugReporter you need to update everything (reporter, client, server). For this just clone/download this repository with updated code and run each script again,
they wil ask you to overwrite the files. Make sure to specify the same parameters you specified when you were installing this for the first time.

# Information

# Information: Server

### Configuration
On first start, the server will create a server configuration file `server_config.ini` next to the executable file.
You can customize values in this config file. In order for them to be applied, restart the server.

The server processes reporters and clients on different ports (see your generated `server_config.ini`).

### Logs

The server will store logs in the `logs` directory. This directory is localed in the place where `server.exe` is located.

# Information: Client

### OTP
When you will login for the first time, the server will request you to scan a QR code with OTP. You have to use an app to scan a QR code for OTPs, for example, Google Authenticator and FreeOTP were confirmed to work correctly with FBugReporter.

### Theme Customization
On the first start, the client will create a theme file `theme.ini` next to the executable file. You can customize values in this theme file. In order for them to be applied, restart the client.

# Build (Manual Installation)

If you don't want or can't use scripts from the `Install` section above, you can build and integrate everything yourself.

## Build: Reporter
**To build** the reporter you will need [Rust](https://www.rust-lang.org/tools/install) and [LLVM](https://github.com/llvm/llvm-project/releases/) installed (when installing LLVM pick "Add LLVM to the system PATH for all users"), then in the `reporter` folder run:

```
cargo build --release
```

The compiled reporter library will be located at `/reporter/target/release/` (with the extension `.dll` for Windows and `.so` for Linux).

**To integrate** the reporter you will need to create a `GDNativeLibrary` resource in your Godot project, you can call it `reporter`. Then navigate to your platform in the opened panel. Click on the folder icon (against number `64` for 64 bit systems, `32` for 32 bit systems) and select the compiled library. Save this resource with the `.gdnlib` extension.

Now create a new script with the following parameters:

- `Language` to `NativeScript`,
- `Inherits` to `Node`,
- `Class Name` to `Reporter`,
- change the name of the script (file) in the `Path` to `reporter`.

Then open this script and in the `Inspector` panel find a property with the name `Library`, click on it, then pick `Load` and select the `reporter.gdnlib` (GDNativeLibrary) file the we created.

See the example project in the `example` folder and `example/MainScene.gd` for how to send reports. You could just copy-paste `MainScene.tscn` and `MainScene.gd` files to your project and customize them as you want.

## Build: Server
**Requirements:**

The server uses SQLite to store data. In order to build the server you need to have `sqlite3` installed.

For Windows users we have a built version of `sqlite3` in `server/sqlite3-windows`. In order to use it, create an environment variable with the name `SQLITE3_LIB_DIR` that points to this directory before building the `server` or `database_manager`.

**Build:**

The server consists of 3 applications:

- `server`: the actual server
- `database_manager`: used to add/remove users (even when the server is running)
- `monitor`: simple helper app that will restart the server if it crashed

You need to build each application and put resulting executable files in the same folder (so that you will have `server`, `database_manager` and `monitor` all in the same folder).

In order to build an app you need to enter its directory and run:

```
cargo build --release
```

The compiled executable be located at `/target/release/`.

Note that Windows users also need to have `sqlite3.dll` library next to the compiled programs, put compiled `server.exe`, `database_manager.exe` and `monitor.exe` to the same directory and copy `sqlite3.dll` from `server/sqlite3-windows` in this folder.
