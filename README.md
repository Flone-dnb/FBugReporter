# FBugReporter

todo

# Build

<h2>Reporter</h2>
<b>In order to build</b> the reporter you will need Rust and LLVM installed, then in the /reporter folder run:
<pre>
cargo build --release
</pre>
Compiled library will be located at /reporter/target/release/ (with the extension ".dll" for Windows and ".so" for Linux)<br>
<br>
<b>In order to integrate</b> the reporter you will need to create a GDNativeLibrary resource in your Godot project, you can call it "reporter". Then navigate to your platform in the opened panel. Click on the folder icon (against number "64" for 64 bit systems, "32" for 32 bit systems) and select the compiled library. Save this resource with the ".gdnlib" extension.<br>
Now create a new script with the following parameters: "Language" to "NativeScript", "Inherits" to "Node", "Class Name" to "Reporter" and change the name of the script (file) in the "Path" to "reporter". Then open this script and in the "Inspector" panel find property with the name "Library", click on it, then pick "Load" and select the reporter.gdnlib (GDNativeLibrary) file the we created.<br><br>
You can then use the following GDScript code to send reports:

```
var reporter = preload(*path to reporter.gdns*).new();

func send_report(
        report_name: String, report_text: String,
        sender_name: String, sender_email: String,
        game_name: String, game_version: String):
    var result_code: int = reporter.send_report(
        report_name, report_text, sender_name, sender_email, game_name, game_version);
    if result_code != 0:
        # notify the user and show him the error code so he can report this issue
        # make sure to include "FBugReporter - reporter.log" which is located
        # Linux: in the folder with the game,
        # Windows: in the Documents folder, in the subfolder "FBugReporter".
        pass

func _notification(what):
    if what == NOTIFICATION_PREDELETE:
        # destructor logic
        reporter.queue_free();
```
