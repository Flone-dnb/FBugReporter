# Example

In order to use this example, you need to build the reporter and get `reporter.dll` (Windows) or `libreporter.so` (Linux).

Go to reporter's folder where `Cargo.toml` resides and execute the following console command:

```
cargo build --release
```

the resulting binary will be located at `target/release`.

- for Windows, put the resulting `reporter.dll` into the directory `example/bin/windows`
- for Linux, put the resulting `libreporter.so` into the directory `example/bin/linux`

Now start the `server_monitor`, it will print the port for reporter connections, we will copy it to our Godot project now.

Open the example project using Godot 4 and change the port in `example/scenes/main.gd` in `func _ready()` on the line:

```
reporter.setup_report_receiver("Server", "127.0.0.1:50123", ""); # where 50123 is your port for reporter from `server_monitor`
```
