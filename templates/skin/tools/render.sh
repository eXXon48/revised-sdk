#!/bin/sh
# Render src/background.svg to the pixel size of the stock PNG it replaces.
# Run in the SDK image (it has rsvg-convert and revised-sdk), from the project directory.
set -e
live=/jci/gui/common/images/background.png
size=$(revised-sdk baseline png-size "$live")
rsvg-convert -w "${size% *}" -h "${size#* }" -o skin/background.png src/background.svg
echo "skin/background.png: $size"
