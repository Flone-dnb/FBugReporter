# FBugReporter

![](screenshot.png?raw=true)

**Project Status**: active (new features as well as bug fixes are on the way).

# Supported Platforms

- Windows
- Linux

# Report Contents

Here is the list of information you can send in the report:

- report name (summary),
- report text (description),
- sender name,
- sender e-mail,
- sender OS info,
- game name,
- game version,
- game screenshot (enabled by default for `example` project),
- attachments (any files).
    - (last 3 log files enabled by default for `example` project).

# Try It Out

Windows users can find built versions of FBugReporter in the [releases](https://github.com/Flone-dnb/FBugReporter/releases) section (with `*_Build.zip` suffix). This archive contains an example Godot project with FBugReporter already integrated, using this example you can send reports. In order to receive reports this archive also has `server_monitor.exe` (that will start the `server.exe`) that you need to start in order to receive reports. To view received reports you can use `client.exe` but it requires an account which you can create after starting the `server_monitor.exe` using the `database_manager.exe` (type `add-user *your name*` to create an account).

The usual workflow goes like this:

- Start the `server_monitor.exe`, it will start the `server.exe` and will restart it if it crashed (thus we should start `server_monitor.exe` instead of explicitly starting the `server.exe`).
- After `server_monitor.exe` was started, run `database_manager.exe` and type command `add-user *your name*` to add a new user (for example: `add-user john`), you will receive new user's password, copy it somewhere.
- Run `client.exe` (`server_monitor.exe` needs to be running), in order to login you need to enter server's IP and port. For local usage put `localhost` as IP. For port look for `server_config.ini` (that will be generated once the `server_monitor.exe` is started), look at `port_for_clients` line that will contain the port you need to use. Now login using the specified earlier name and the password you received, after this you will change the password and setup OTP. After everything is done you will see received reports.
- To generate new reports, open Godot project with FBugReporter integrated (build version from `releases` already has a project with FBugReporter integrated) and send a report (while `server_monitor.exe` is running). You can then see new reports in `client.exe` (if you don't see new reports, use `Refresh Report List` button).

# Attachments

Once you run the server for the first time, the server will generate `server_config.ini` that you can customize. There is a specific setting for attachments `max_total_attachment_size_in_mb` which specifies the maximum size in megabytes of report attachments (in total - for all files, not per file).

By default its value is 5 MB which means that you can attach any files as long as their total size is not bigger than 5 MB.

To tell if your attachments are too big or not, reporter's `send_report` function will ask the server for maximum allowed attachment size, calculate the total size of the specified attachments and if attachments exceed the maximum limit reporter's `send_report` function will return error code '9' (see `example` directory for more information).

# How to Install

If you tried the built version from `releases` and now want to integrate FBugReporter into your Godot game follow this section.

To make the process of installation easier, I wrote a few scripts that you can find in the `install` directory of this repository.

In order to run these scripts you need to have [Go](https://go.dev/dl/) installed. To run a script open a directory with the `*.go` file, and type `cmd` in the explorer's address bar, once the console is opened, type `go run .` in order to start the script.

- `install_client.go` this script will ask you where you want the client to be installed and will compile the client. It will also check if needed dependencies are installed and if not will prompt you to install them.
- `install_reporter.go` this script will ask you where your Godot project is located and will compile and integrate reporter into your project (with premade UI scene to send reports). It will also check if needed dependencies are installed and if not will prompt you to install them.
- `install_server.go` this script will ask you where you want the server to be installed and will compile the server, server monitor and database manager for you. It will also check if needed dependencies are installed and if not will prompt you to install them.

If you want to integrate FBugReporter into your Godot game, just clone/download this repository and run each script (the order does not matter), they will setup everything for you.

Once everything is done. Your project will have new files, our main interest is `reporter.gd` file, it uses premade `reporter.tscn` scene to send reports, you can customize everything as you want, you can even delete these files and start making all from scratch. But now let's focus on sending your first report.

First up, start `server_monitor` that was installed using `install_server.go`, it will generate `server_config.ini` file. This file contains server configuration that can be customized. Note that in order for changed values to be applied `server_monitor` restart is required. Now, in `server_config.ini` look for the parameter `port_for_reporters`. You need to put this value into `reporter.gd` file. In your Godot project find `reporter.gd` and look for `_ready()` function inside of it. There will be a line that looks something like this: `reporter.set_server("localhost", 21580)`. Now change the second argument of `set_server` function to your `port_for_reporters` from `server_config.ini`. That's it! Start `server_monitor`, run generated `reporter.tscn` scene from your project and send your first report!

In order to view reports you first need an account. Run `database_manager` that was installed using `install_server.go`. After `database_manager` is started, type command `add-user <your name>` (for example, `add-user john`). You will then be asked about new user's privileges and will receive new user's password, remember it. Exit from `database_manager` using `exit` command. Open up `client` application that was installed using `install_client.go`. We now need to enter server information. For `server` type `localhost`, for `port` type value `port_for_clients` from `server_config.ini` (not `port_for_reporters`!), for `username` type the username you used in `add-user` command, for `password` use password that you received in `database_manager`. Now try to login, you will go through the first login process and will setup your new password and OTP. After everything is done you will see reports that the server received from your game!

The database that stores all information (reports, registered users, attachments and etc.) is called `database.db3` and it was generated when you run `server_moninor` for the first time. If you want to backup your database you just need to copy this `database.db3` file - there are no other dependencies, just make a copy of your `database.db3` file and that's it - that's your backup.

### About dedicated servers

For development or testing you can run server on your computer. When you want to share your game with somebody you need to make sure that the server's IP address is not going to change. So, let's consider this situation: you've added `server_monitor` to autostart in your computer, your game uses `localhost` as server IP (as we've seen in `reporter.gd`), you give your game to your friend and... he won't be able to send reports. Because the game sends reports to `localhost` (which is an alias for "this computer" or "local computer") your friend will send reports to his own computer but he does not have a running server and moreover this is not what we want. You may try to replace `localhost` with your public IP address but the thing is that when you restart your network router your provider usually gives you a new public IP address, some providers even give you new public IP address from time to time (at night for example) even if you wont restart your router. Thus, when you are ready to share your build with somebody you need to install the server to a dedicated server (a cloud server or "VPS/VDS" as they are called) that will have a static IP address which you will specify in your game. There a numerous "VPS/VDS" hosters with cheap hosting. For example, I use Hetzner's cloud server for this purpose. You don't need to have a Windows installed on your cloud server (there is no need for that, moreover Windows servers are usually more expensive), you can buy a cheap Linux server and install FBugReporter's server there (using `install_server.go`, because you will install it on Linux the script will also automatically add everything to autostart). The only things that you will need to do is install `Rust`, `Go` and `sqlite` packages, then run `install_server.go` and restart. Then you can use your server's static IP address in your game. Because setting up your own dedicated server might be a complicated task (if you've never tried this before or don't have any Linux experience) that can include multiple subtasks (like disabling root login, setting up a firewall, seting up a proper ssh connection and fail2ban) I will not talk about it here because it's a different topic. There are lots of tutorials on YouTube that will help you setup your own VPS/VDS. Once you have a Linux VPS/VDS, all you need to do is install `Rust`, `Go` and `sqlite` packages to the system and install FBugReporter's server using `install_server.go`.

# How to Update

If you want to update FBugReporter you need to update everything (reporter, client, server). For this just clone/download this repository with updated code and run each script again,
they will ask you to overwrite the files. Make sure to specify the same parameters you specified when you were installing this for the first time.

# Information

# Information: Server

### Configuration
On first start, the server will create a server configuration file `server_config.ini` next to the executable file.
You can customize values in this config file. In order for them to be applied, restart the server.

The server processes reporters and clients on different ports (see your generated `server_config.ini`).

### Logs

The server will store logs in the `logs` directory. This directory is localed in the directory where `server.exe` is located.

# Information: Client

### OTP
When you will login for the first time, the server will request you to scan a QR code with OTP. You have to use an app to scan a QR code for OTPs, for example, Google Authenticator and FreeOTP were confirmed to work correctly with FBugReporter.

### Theme Customization
On the first start, the client will create a theme file `theme.ini` next to the executable file. You can customize values in this theme file. In order for them to be applied, restart the client.

# Build (Manual Installation)

If you don't want or can't use scripts from the `How to Install` section above, you can build and integrate everything yourself.

## Build: Reporter
**To build** the reporter you will need [Rust](https://www.rust-lang.org/tools/install) and [LLVM](https://github.com/llvm/llvm-project/releases/) installed (when installing LLVM pick "Add LLVM to the system PATH for all users"), then in the `reporter` directory run:

```
cargo build --release
```

The compiled reporter library will be located at `/reporter/target/release/` (with the extension `.dll` for Windows and `.so` for Linux).

**To integrate** the reporter you will need to create a `GDNativeLibrary` resource in your Godot project, you can call it `reporter`. Then navigate to your platform in the opened panel. Click on the directory icon (against number `64` for 64 bit systems, `32` for 32 bit systems) and select the compiled library. Save this resource with the `.gdnlib` extension.

Now create a new script with the following parameters:

- `Language` to `NativeScript`,
- `Inherits` to `Node`,
- `Class Name` to `Reporter`,
- change the name of the script (file) in the `Path` to `reporter`.

Then open this script and in the `Inspector` panel find a property with the name `Library`, click on it, then pick `Load` and select the `reporter.gdnlib` (GDNativeLibrary) file the we created.

See the example project in the `example` directory and `example/MainScene.gd` for how to send reports. You could just copy-paste `MainScene.tscn` and `MainScene.gd` files to your project and customize them as you want.

## Build: Server
**Requirements:**

To build the server you will need [Rust](https://www.rust-lang.org/tools/install).
The server uses SQLite to store data. In order to build the server you also need have `sqlite3` installed.

For Windows users we have a built version of `sqlite3` in `server/sqlite3-windows`. In order to use it, create an environment variable with the name `SQLITE3_LIB_DIR` that points to this directory before building the `server` or `database_manager`.

**Build:**

The server consists of 3 applications:

- `server`: the actual server
- `database_manager`: used to add/remove users (even when the server is running)
- `server_monitor`: simple helper app that will restart the server if it crashed

You need to build each application and put resulting executable files in the same directory (so that you will have `server`, `database_manager` and `server_monitor` all in the same directory).

In order to build an app you need to enter its directory and run:

```
cargo build --release
```

The compiled executable be located at `/target/release/`.

Note that Windows users also need to have `sqlite3.dll` library next to the compiled programs, put compiled `server.exe`, `database_manager.exe` and `server_monitor.exe` to the same directory and copy `sqlite3.dll` from `server/sqlite3-windows` in this directory.

## Build: Client

To build the client you will need [Rust](https://www.rust-lang.org/tools/install).

Then in the `client` directory run:

```
cargo build --release
```

The compiled client binary will be located at `/client/target/release/`.
