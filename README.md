This is a WIP command line tool to help with downloading playlists from yt-dlp.

```
Usage: pl-update.exe [OPTIONS] <COMMAND>

Commands:
  init    Creates a new directory for a playlist and downloads all associated songs
  update  Checks playlist for new or removed songs, and downloads/deletes files respectively. Requires a valid playlist-settings.json file   
  repair  Rebuilds the playlist manifest from the files in the directory. Requires a playlist manifest containing at least the playlist url  
  push    Will send the files in the playlist to a connected media device (Android phone, music player) etc.. If device id is not specified, and there is more than one device connected will prompt user to select device
  help    Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose
          Print extra debugging information
  -q, --quiet
          Suppress output. Also disables interactive promps
      --suppress-interactive
          Disable interactive prompts
      --yt-dl-args <YT_DL_ARGS>
          Args to pass to yt-dlp
      --yt-dl-location <YT_DL_LOCATION>
          The location of yt-dlp [default: yt-dlp]
      --postproccessor-args <POSTPROCCESSOR_ARGS>
          Args provided to ffmpeg to run on every file after it is downloaded not implemented
  -t, --threads <THREADS>
          The number of threads to use [default: 2]
  -h, --help
          Print help
  -V, --version
          Print version
```
