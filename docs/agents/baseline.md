## Stock baseline

The SDK needs the stock `74.00.324A` files to check your project against. This project never ships them. It publishes only a path and md5 list (`baseline/74.00.324A.md5`) and the steps below. You extract the baseline from the official update file (`.up`) that you already have.

The `.up` file is a password-protected ZIP. The password is not published here. It is well known in the CMU community, and it is stored in the stock firmware: look for a 16-character string in `libsecurity.so`. Set it in `CMU_UP_PASS` before you run the extraction.

**Why it is not published here.** The password belongs to the vendor's update format, not to this project. In some countries, publishing a key that opens a protected file can be treated as distributing a circumvention tool, even when the key is already public. A takedown would hit the whole repository, not one line. So this project ships only its own work: the extraction steps, file lists and hashes, never the key and never vendor files.

Extraction gives you a read-only stock baseline for diffs, lint and preview. It does not let you build firmware: an unpacked, modified and repacked `.up` is rejected by the unit's update check. Revised packages never modify or redistribute `.up` files.

### Extract

Use Linux, WSL or the SDK container. Do not extract on NTFS: it loses modes, owners and symlinks.

```sh
unzip -P "$CMU_UP_PASS" -p <ver>_update.up rootfs1upd/e0000000001.dat.gz | gunzip | gunzip > rootfs.tar
sudo mkdir -p stock/rootfs
sudo tar -xpf rootfs.tar --numeric-owner -C stock/rootfs
revised-sdk baseline check stock/rootfs
```

`baseline check` compares every listed file with the published md5 and reports `baseline-missing` and `baseline-mismatch` findings. It records the path in `.revised/local.toml`. A baseline that does not match is not genuine, unmodified `74.00.324A`: do not use it.

### Mount read-only

- SDK container: bind it as `-v ./stock/rootfs:/rootfs:ro`.
- As an image: `mkfs.ext4 -d stock/rootfs rootfs.img 3G`, then `mount -o loop,ro,norecovery rootfs.img <dir>`.
- Windows: `wsl --mount <vhd> --vhd --bare`, then `mount -o ro,norecovery <device> <dir>`.

Search stock files only there. Never search the unit.
