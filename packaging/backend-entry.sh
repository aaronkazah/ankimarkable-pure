#!/bin/sh
set -eu
app=/home/root/xovi/exthome/appload/ankimarkable-pure
export AM_MATHPNG="$app/mathpng"
export AM_MATHPNG_RES="$app/microtex-res"
exec "$app/ankimarkable-pure" "$1"
