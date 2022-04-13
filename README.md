# FBugReporter

TODO

# Server

### Configuration
On first start, the server will create a server configuration file `server_config.ini` next to the binary file.
You can customize values in this config file. In order for them to be applied, restart the server.

The server processes reporters and clients on different ports (see your generated `server_config.ini`).

# Client

### OTP
When you will login for the first time, the server will request you to scan a QR code with OTP. You have to use an app to scan a QR code for OTPs, for example, Google Authenticator and FreeOTP were cofirmed to work correctly with FBugReporter.

### Theme Customization
On first start, the client will create a theme file `theme.ini` next to the binary file. You can customize values in this theme file. In order for them to be applied, restart the client.

# Build

## 1. Reporter
<b>To build</b> the reporter you will need [Rust](https://www.rust-lang.org/tools/install) and [LLVM](https://github.com/llvm/llvm-project/releases/) installed (when installing LLVM pick "Add LLVM to the system PATH for all users"), then in the `reporter` folder run:

```
cargo build --release
```

Compiled reporter library will be located at `/reporter/target/release/` (with the extension `.dll` for Windows and `.so` for Linux).

<b>To integrate</b> the reporter you will need to create a GDNativeLibrary resource in your Godot project, you can call it `reporter`. Then navigate to your platform in the opened panel. Click on the folder icon (against number `64` for 64 bit systems, `32` for 32 bit systems) and select the compiled library. Save this resource with the `.gdnlib` extension.

Now create a new script with the following parameters:

- `Language` to `NativeScript`,
- `Inherits` to `Node`,
- `Class Name` to `Reporter`,
- change the name of the script (file) in the `Path` to `reporter`.

Then open this script and in the `Inspector` panel find property with the name `Library`, click on it, then pick `Load` and select the `reporter.gdnlib` (GDNativeLibrary) file the we created.

See the example project in the `example` folder and `example/MainScene.gd` for how to send reports. You could just copy-paste `MainScene.tscn` and `MainScene.gd` files to your project and customize them as you want.

## 2. Server
<b>Requirements</b>
The server uses SQLite to store data. In order to build the server you need to have `sqlite3` installed.

Windows users are special 🙃, they need to build sqlite library in order to build the server, you can use the following guide for example: https://gist.github.com/zeljic/d8b542788b225b1bcb5fce169ee28c55
