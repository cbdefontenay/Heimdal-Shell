# Heimdal-Shell

This is a simple shell with a few customized commands. The core difference between Heimdal and other Shells like ```Bash``` is that Heimdal allows you to chat with someone else directly inside the Shell.

It's secured, at least as much as I could. To enter a chat you'll need to give the command a name + port + password like:

```bash
chat host mychat 9898 mysecretp@ssw0rd
```

Else just enter ```heimdal --help``` to know more about those commands.

#### Steps to follow using ```cargo```:

```rust
cargo install cargo-deb
```

Then:
```bash
cd to the Heimdal --bin project directory
```
Then:
```rust
cargo deb
```

You'll find the .deb file at this location:
```
target/debian/heimdal-shell_<version>_amd64.deb
```

In order to install Heimdal system-wide:
```bash
sudo dpkg -i target/debian/heimdal-shell_<version>_amd64.deb
```