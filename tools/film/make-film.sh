#!/usr/bin/env bash
# Makes the film of Star Racer (docs/film.md): records two takes against a local server, writes
# the acid techno, and cuts the takes on the music's bars. Run from the repository's root, with the
# workspace built, the server running on 127.0.0.1 from a fresh start (its quick play lobbies
# counting from 0), and ffmpeg on the PATH or in FFMPEG.
#
#     bash tools/film/make-film.sh
#
# Out: target/film/star-racer-film.mp4. Only what the game renders is recorded (`--record`), never
# the screen around it.
set -euo pipefail

FF=${FFMPEG:-ffmpeg}
OUT=target/film
CLIENT=target/debug/star-racer-client.exe
mkdir -p "$OUT"
rm -rf "$OUT/a" "$OUT/b"

# Take A: from the first screen, QUICK PLAY pressed after 4 s, the race driven by the autopilot.
"$CLIENT" --server 127.0.0.1 --identity target/id-film --press-quick-play 4 --autopilot-drift \
    --volume 0 --window-size 1280x720 --record "$OUT/a" --record-fps 30 --record-seconds 130

# Take B: another quick play race, filled with bots, watched by a spectator whose camera changes
# on its own.
"$CLIENT" --server 127.0.0.1 --identity target/id-racer --guest --quick-play --autopilot-drift \
    --volume 0 --window-size 640x360 &
racer=$!
sleep 5
"$CLIENT" --server 127.0.0.1 --identity target/id-watch --guest --watch "Quick Play 1" \
    --camera orbit --camera-cycle 9 --volume 0 --window-size 1280x720 \
    --record "$OUT/b" --record-fps 30 --record-seconds 85
kill "$racer" || true

# Each take as a 30 fps video, every frame shown until the next one's time.
for take in a b; do
    ls "$OUT/$take" | sort | awk -v d="$take" '
        { t = substr($0, 7, 8) + 0; if (prev != "") printf "file %s/%s\nduration %.4f\n", d, prev, (t - pt) / 1000.0; prev = $0; pt = t }
        END { printf "file %s/%s\nduration 0.0333\nfile %s/%s\n", d, prev, d, prev }' > "$OUT/$take.txt"
    (cd "$OUT" && "$FF" -y -loglevel error -f concat -safe 0 -i "$take.txt" \
        -vf "fps=30,format=yuv420p" -c:v libx264 -preset fast -crf 16 "$take.mp4")
done

# The music.
rustc -O tools/film/acid.rs -o "$OUT/acid.exe"
"$OUT/acid.exe" "$OUT/acid.wav"

# The cut: tools/film/cuts.txt lists take, start in seconds and length in bars of 132 BPM.
cp client/assets/fonts/Saira-Italic.ttf "$OUT/saira.ttf"
BAR=1.818182
calc() { awk "BEGIN { printf \"%.4f\", $1 }"; }
filters=""; inputs=""; n=0
while read -r take start bars; do
    len=$(calc "$bars * $BAR")
    filters="${filters}[$n:v]trim=start=0:duration=${len},setpts=PTS-STARTPTS,fps=30,format=yuv420p[v$n];"
    inputs="$inputs -ss $start -t $(calc "$len + 0.5") -i $take.mp4"
    n=$((n + 1))
done < tools/film/cuts.txt
concat=""; for i in $(seq 0 $((n - 1))); do concat="${concat}[v$i]"; done
total=$(awk -v b=$BAR '{ s += $3 } END { printf "%.3f", s * b }' tools/film/cuts.txt)
fade=$(calc "$total - 2.5"); title=$(calc "$total - 5.0"); url=$(calc "$title + 0.6")
cd "$OUT"
# shellcheck disable=SC2086
"$FF" -y -loglevel error $inputs -i acid.wav -filter_complex "${filters}${concat}concat=n=$n:v=1:a=0[cat];\
[cat]drawbox=x=0:y=ih/2-150:w=iw:h=290:color=black@0.62:t=fill:enable='gte(t,$title)',\
drawtext=fontfile=saira.ttf:text='STAR RACER':fontsize=130:fontcolor=white:borderw=3:bordercolor=white:shadowcolor=0x1E9BFF:shadowx=7:shadowy=7:x=(w-text_w)/2:y=(h-text_h)/2-45:alpha='if(lt(t,$title),0,min(1,(t-$title)/0.8))',\
drawtext=fontfile=saira.ttf:text='star-racer.xyz':fontsize=44:fontcolor=0x9FD8FF:borderw=1:bordercolor=0x9FD8FF:x=(w-text_w)/2:y=(h/2)+70:alpha='if(lt(t,$url),0,min(1,(t-$url)/0.8))',\
fade=t=in:st=0:d=0.6,fade=t=out:st=$fade:d=2.5[v];[$n:a]atrim=0:$total,afade=t=out:st=$fade:d=2.5[a]" \
    -map "[v]" -map "[a]" -c:v libx264 -preset slow -crf 18 -pix_fmt yuv420p -c:a aac -b:a 192k \
    -movflags +faststart star-racer-film.mp4
rm -rf a b a.txt b.txt
echo "$OUT/star-racer-film.mp4"
