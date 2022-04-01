# Example

In order to use this example, you need to build reporter and get <b>reporter.dll</b> (Windows) or <b>reporter.so</b> (Linux).<br>
<br>
Go to reporter's folder where <b>Cargo.toml</b> resides and execute the following console command:
<pre>
cargo build --release
</pre>
resulting binary will be located at <b>target/release/</b><br>
<br>
- for Windows, create a new folder <b>bin</b> in the folder of the example project (next to the <b>lib</b> folder), then in <b>bin</b> create <b>win64</b> folder and put <b>reporter.dll</b> there,<br>
- for Linux, create a new folder <b>bin</b> in the folder of the example project (next to the <b>lib</b> folder), then in <b>bin</b> create <b>x11_64</b> folder and put <b>reporter.so</b> there,<br>
<br>
open project in Godot and in "FileSystem" panel open folder <b>lib</b> and file <b>reporter.gdnlib</b>, this will show a new panel in which you need to check that "Dynamic Library" path is correct for your platform (Windows/Linux).<br>
<br>
After that, you need to start the server or monitor and change the <b>port</b> in <b>MainScene.gd</b> in <b>func _ready()</b> on the line:
<pre>
reporter.set_server(127, 0, 0, 1, 50123); # where 50123 is your port
</pre>
according to the port your server uses (will be printed when the server is started).
