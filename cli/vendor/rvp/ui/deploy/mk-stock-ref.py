#!/usr/bin/env python3
"""mk-stock-ref.py <up-file> <out-dir> - stock rootfs reference from a CMU .up.

Run: CMU_UP_PASS=... python3 -I mk-stock-ref.py <up> <out-dir>
Writes <fw>.meta ("size mtime /path", sorted), <fw>.md5 ("md5  /path"),
<fw>.exclude (empty). Metadata only: no firmware bytes leave the .up.
"""
import gzip, os, re, sys, tarfile, zipfile

TAR = "rootfs1upd/e0000000001.dat.gz"   # gzip around rootfs.tar.gz
MD5 = "rootfs1upd/e0000000002.dat.gz"   # rootfs.md5, paths under /tmp/rootfs/


def main(up, out):
    pw = os.environ.get("CMU_UP_PASS")
    if not pw:
        sys.exit("CMU_UP_PASS not set")
    z = zipfile.ZipFile(up)
    pw = pw.encode()

    rows, links, ini = [], [], None
    with gzip.open(z.open(TAR, pwd=pw)) as raw, tarfile.open(fileobj=raw, mode="r|gz") as tf:
        for m in tf:
            path = "/" + m.name.removeprefix("./").lstrip("/")
            if m.isreg():  # symlinks, dirs, hard links excluded
                rows.append((path, m.size, int(m.mtime)))
            if m.issym():
                if " /" in path:  # the stick parses "<target> /path" by the last " /"
                    sys.exit("symlink path contains ' /': " + path)
                links.append((path, m.linkname))
            if path == "/jci/version.ini":
                ini = tf.extractfile(m).read()
    if not ini:
        sys.exit("jci/version.ini not found")

    # fw = number + patch letter: JCI_SW_VER="MAZ_CMU-150_74.00.324" + JCI_SW_VER_PATCH="A"
    def key(k):
        return re.search(rb'^' + k + rb'="?([^"\r\n]*)', ini, re.M).group(1).decode()
    fw = key(b"JCI_SW_VER").rsplit("_", 1)[-1] + key(b"JCI_SW_VER_PATCH")
    rows.sort()

    md5 = gzip.decompress(z.read(MD5, pwd=pw)).decode().replace("/tmp/rootfs/", "/")

    os.makedirs(out, exist_ok=True)
    with open(f"{out}/{fw}.meta", "w", newline="\n") as f:
        f.write("".join(f"{s} {t} {p}\n" for p, s, t in rows))
    with open(f"{out}/{fw}.md5", "w", newline="\n") as f:
        f.write(md5.replace("\r\n", "\n"))
    links.sort()
    with open(f"{out}/{fw}.links", "w", newline="\n") as f:
        f.write("".join(f"{t} {p}\n" for p, t in links))
    open(f"{out}/{fw}.exclude", "w").close()  # filled by the bench open item
    print(fw, len(rows), sum(r[1] for r in rows), "links", len(links))


if len(sys.argv) != 3:
    sys.exit(__doc__)
main(*sys.argv[1:])
