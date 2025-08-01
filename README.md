
# ORMSeparateV3

Easily separate RGB channels with Regex naming and other file formats.

## Installation

Cargo is used for installation. You can either run
```sh
$ cargo install ormseparatev3
```
or download directly from releases (recommended)
## Configuration

By default, after running the binary; it will create a config file (toml) if it doesn't already exist in the binary location. The configuration is simple.
```toml
default_profile = "orm"

[profiles.orm]
name = "orm"
file_regex = "/orm/i"
output_format = "png"

[[profiles.orm.channels]]
name = "Occlusion"
channel = 0

[[profiles.orm.channels]]
name = "Roughness"
channel = 1

[[profiles.orm.channels]]
name = "Metallic"
channel = 2
```
You should get it now.