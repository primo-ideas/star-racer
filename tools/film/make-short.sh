#!/usr/bin/env bash
# Cuts the short film of Star Racer (docs/film.md#the-short): a grid of sixteen cars, no menu, about
# 36 seconds at 1080p, under Discord's 20 MB. Takes the two takes recorded in target/film2 (a.mp4,
# the racer from the grid; b.mp4, the spectator) and the acid techno of make-film.sh
# (target/film/acid.wav), and cuts them on its bars by tools/film/short-cuts.txt.
#
#     bash tools/film/make-short.sh
#
# Out: target/film2/star-racer-short.mp4.
set -euo pipefail

FF=${FFMPEG:-ffmpeg}
OUT=target/film2
BAR=1.818182
# The music from bar 4, where the kick comes in.
MUSIC_START=7.2727
VIDEO_RATE=3800k

[ -f target/film/acid.wav ] || { rustc -O tools/film/acid.rs -o target/film/acid.exe && target/film/acid.exe target/film/acid.wav; }
cp target/film/acid.wav "$OUT/acid.wav"
cp client/assets/fonts/Saira-Italic.ttf "$OUT/saira.ttf"

calc() { awk "BEGIN { printf \"%.4f\", $1 }"; }
filters=""; inputs=""; n=0
while read -r take start bars; do
    len=$(calc "$bars * $BAR")
    filters="${filters}[$n:v]trim=start=0:duration=${len},setpts=PTS-STARTPTS,fps=30,format=yuv420p[v$n];"
    inputs="$inputs -ss $start -t $(calc "$len + 0.5") -i $take.mp4"
    n=$((n + 1))
done < tools/film/short-cuts.txt
concat=""; for i in $(seq 0 $((n - 1))); do concat="${concat}[v$i]"; done
total=$(awk -v b=$BAR '{ s += $3 } END { printf "%.3f", s * b }' tools/film/short-cuts.txt)
fade=$(calc "$total - 1.8"); title=$(calc "$total - 4.0"); url=$(calc "$title + 0.6")
graph="${filters}${concat}concat=n=$n:v=1:a=0[cat];\
[cat]drawbox=x=0:y=ih/2-210:w=iw:h=420:color=black@0.62:t=fill:enable='gte(t,$title)',\
drawtext=fontfile=saira.ttf:text='STAR RACER':fontsize=190:fontcolor=white:borderw=4:bordercolor=white:shadowcolor=0x1E9BFF:shadowx=10:shadowy=10:x=(w-text_w)/2:y=(h-text_h)/2-65:alpha='if(lt(t,$title),0,min(1,(t-$title)/0.6))',\
drawtext=fontfile=saira.ttf:text='star-racer.xyz  ·  alpha':fontsize=64:fontcolor=0x9FD8FF:borderw=1:bordercolor=0x9FD8FF:x=(w-text_w)/2:y=(h/2)+100:alpha='if(lt(t,$url),0,min(1,(t-$url)/0.6))',\
fade=t=in:st=0:d=0.4,fade=t=out:st=$fade:d=1.8[v];\
[$n:a]atrim=start=$MUSIC_START:duration=$total,asetpts=PTS-STARTPTS,afade=t=in:st=0:d=0.2,afade=t=out:st=$fade:d=1.8[a]"

cd "$OUT"
# Two passes, so the size is what the rate says: (3.8 + 0.16) Mb/s over 36 s is about 18 MB.
# shellcheck disable=SC2086
"$FF" -y -loglevel error $inputs -i acid.wav -filter_complex "$graph" -map "[v]" -map "[a]" -c:a aac \
    -c:v libx264 -preset slow -b:v $VIDEO_RATE -pass 1 -passlogfile short -f null NUL
# shellcheck disable=SC2086
"$FF" -y -loglevel error $inputs -i acid.wav -filter_complex "$graph" -map "[v]" -map "[a]" \
    -c:v libx264 -preset slow -b:v $VIDEO_RATE -maxrate 6000k -bufsize 8000k -pass 2 -passlogfile short \
    -pix_fmt yuv420p -c:a aac -b:a 160k -movflags +faststart star-racer-short.mp4
rm -f short-0.log short-0.log.mbtree
echo "$OUT/star-racer-short.mp4"
