# FBugReporter

todo

# Server

<h3>Requirements</h3>
The server uses SQLite to store data. In order to build the server you need to have <b>sqlite3</b> installed.<br>
Windows users are special 🙃, they need to build sqlite library in order to build the server, you can use [this guide](https://gist.github.com/zeljic/d8b542788b225b1bcb5fce169ee28c55) for example.
<h3>Configuration</h3>
On first start, the server will create a server configuration file <b>server_config.ini</b> next to the binary file.<br>
You can customize values in this config file. In order for them to be applied, restart the server.<br>
<br>
The server processes reporters and clients on different ports (see your generated <b>server_config.ini</b>).

# Client

On first start, the client will create a theme file <b>theme.ini</b> next to the binary file.<br>
You can customize values in this theme file. In order for them to be applied, restart the client.

# Build

<h2>Reporter</h2>
<h4>To build</h4> the reporter you will need Rust and LLVM installed, then in the /reporter folder run:
<pre>
cargo build --release
</pre>
Compiled library will be located at /reporter/target/release/ (with the extension ".dll" for Windows and ".so" for Linux)<br>
<br>
<h4>To integrate</h4> the reporter you will need to create a GDNativeLibrary resource in your Godot project, you can call it "reporter". Then navigate to your platform in the opened panel. Click on the folder icon (against number "64" for 64 bit systems, "32" for 32 bit systems) and select the compiled library. Save this resource with the ".gdnlib" extension.<br>
Now create a new script with the following parameters: "Language" to "NativeScript", "Inherits" to "Node", "Class Name" to "Reporter" and change the name of the script (file) in the "Path" to "reporter". Then open this script and in the "Inspector" panel find property with the name "Library", click on it, then pick "Load" and select the reporter.gdnlib (GDNativeLibrary) file the we created.<br><br>
See the example project in the <b>example</b> folder and <b>example/MainScene.gd</b> for how to send reports. You could just copy-paste <b>MainScene.tscn</b> and <b>MainScene.gd</b> files to your project and customize them as you want.
