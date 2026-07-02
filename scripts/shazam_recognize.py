import asyncio, json, sys, traceback
from shazamio import Shazam, Serialize

async def main():
    wav_path = sys.argv[1]
    shazam = Shazam()
    try:
        result = await shazam.recognize(wav_path)
    except Exception:
        print(json.dumps({"error": traceback.format_exc()}))
        return

    track = result.get("track") or {}
    if not track.get("title"):
        print(json.dumps({"matches": []}))
        return

    title = track.get("title", "Unknown")
    artist = track.get("subtitle", "Unknown")
    images = track.get("images") or {}
    hub = track.get("hub") or {}
    actions = hub.get("actions") or []
    preview_url = None
    for a in actions:
        if a.get("type") == "uri":
            preview_url = a.get("uri")
            break
    matches = track.get("matches") or []
    score = matches[0].get("score", 0.8) / 100.0 if matches else 0.8
    sections = track.get("sections") or []
    metadata = {}
    for s in sections:
        if s.get("type") == "SONG":
            for m in s.get("metadata") or []:
                metadata[m.get("title", "")] = m.get("text", "")
    isrc = track.get("isrc") or ""

    output = {
        "title": title,
        "artist": artist,
        "album": metadata.get("Album", ""),
        "artwork_url": images.get("coverarthq") or images.get("coverart"),
        "preview_url": preview_url,
        "score": min(score, 1.0),
        "isrc": isrc,
        "duration": None,
    }
    print(json.dumps(output))

asyncio.run(main())
