# Music player
Simple ncmpcpp-inspired mpd player written in rust
<img width="1280" height="705" alt="image" src="https://github.com/user-attachments/assets/42b0c217-fa78-446c-8e44-2f1f8d88743a" />

## Usage
Create a configuration file in `~/.config/music-player/config.toml`. Can just take it off ncmpcpp. No default is implemented yet, but this is mine
```toml
mpd_host = "localhost"
mpd_port = 6600
mpd_music_dir = "~/Music"
visualizer_data_source = "/tmp/mpd.fifo"
visualizer_output_name = "my_fifo"
visualizer_in_stereo = "yes"
visualizer_type = "spectrum"
visualizer_look = "+|"
```


## Commands
Press `:` to open the command bar.
- `:a album-name` will start autocomplete for albums
- `:s song-name` will start autocomplete for songs

## Keybinds
- `j` and `k` move up and down.
- `h` and `l` move a level deeper (artist <=> album <=> songs)
- `/` start search. `n`/`N` to go next/prev.
- `<C-u>` and `<C-d>` go up and down 10 lines at once.
- `1` Playlist view
- `3` Search
- `4` Library

Search using `/`
<img width="1280" height="705" alt="image" src="https://github.com/user-attachments/assets/538fdb80-5a82-40b1-8932-5b69a7f0622c" />
