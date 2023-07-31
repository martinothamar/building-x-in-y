# Notes

Various notes

### [Installing `perf` on WSL2](https://scicoding.com/how-to-perform-perf-profiling-in-wsl2/)

```sh
sudo apt install linux-tools-generic
ls /usr/lib/linux-tools/ # <- something should be installed here
# In my case, the perf binary was installed here
stat /usr/lib/linux-tools/5.15.0-76-generic/perf

# If there is a preinstalled perf in PATH, back it up
sudo mv /usr/bin/perf /usr/bin/perf.bak

# We can then just symlink it to PATH
sudo ln -s /usr/lib/linux-tools/5.15.0-76-generic/perf /usr/bin/perf
```

### Install newer version of WSL 2 Linux kernel

* Download archive from https://github.com/microsoft/WSL2-Linux-Kernel/releases
* Follow build instructions here: https://github.com/microsoft/WSL2-Linux-Kernel#build-instructions
  * `make KCONFIG_CONFIG=Microsoft/config-wsl -j8` - remember j flag
* Copy the kernel somewhere
  * `mv vmlinux* /mnt/c/WSL2Kernel/`
* Update `C:\Users\<UserName>\.wslconfig`

```toml
[wsl2]
kernel=C:\\WSL2Kernel\\vmlinux
```

