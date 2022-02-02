# FBugReporter

todo

# Client

Theme can be customized by copy-pasting the <b>theme.ini</b> file from <b>client</b> folder next to the client executable file.<br>
If there is a file <b>theme.ini</b> next to the client binary it will read theme values from it instead of using default values.

# Build

<h2>Reporter</h2>
<b><u>In order to build</u></b> the reporter you will need Rust and LLVM installed, then in the /reporter folder run:
<pre>
cargo build --release
</pre>
Compiled library will be located at /reporter/target/release/ (with the extension ".dll" for Windows and ".so" for Linux)<br>
<br>
<b><u>In order to integrate</u></b> the reporter you will need to create a GDNativeLibrary resource in your Godot project, you can call it "reporter". Then navigate to your platform in the opened panel. Click on the folder icon (against number "64" for 64 bit systems, "32" for 32 bit systems) and select the compiled library. Save this resource with the ".gdnlib" extension.<br>
Now create a new script with the following parameters: "Language" to "NativeScript", "Inherits" to "Node", "Class Name" to "Reporter" and change the name of the script (file) in the "Path" to "reporter". Then open this script and in the "Inspector" panel find property with the name "Library", click on it, then pick "Load" and select the reporter.gdnlib (GDNativeLibrary) file the we created.<br><br>
See the example project in the <b>example</b> folder and <b>example/MainScene.gd</b> for how to send reports. You could just copy-paste <b>MainScene.tscn</b> and <b>MainScene.gd</b> files to your project and customize them as you want.
