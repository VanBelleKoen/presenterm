## Dual terminal mode

The `--dual` (`-d`) parameter lets you present from one terminal while showing the audience a second, clean one. This is
useful when the second terminal is on a projector or a second display.

```bash
presenterm demo.md --dual
```

This opens a new window of the terminal you're running in and turns the current terminal into the _master_:

* The **master** (the terminal you ran the command in) shows the current slide, the
[speaker notes](speaker-notes.md) at the bottom and a smaller preview of the next slide on the right. This is the one
you control. The slides use your theme, but the notes panel and its labels have a fixed dark look that ignores it: the
notes are shown as plain word-wrapped text across the full width, without slide titles, so they stay readable
whatever theme you use. The preview of the next slide always shows the slide in full, ignoring pauses and incremental lists.
* The **slave** (the new window) only shows the current slide. It is display only: its keys can't change the slide, you
can only quit it.

Any navigation you do in the master (next/previous, jumping to a slide, going through pauses, etc) is mirrored in the
slave. Each terminal renders the presentation at its own size.

When the master quits the slave closes automatically. If you close the slave, the master keeps working. If the master
disappears without closing cleanly, the slave exits after a few seconds.

Since the master has less room than a regular presentation, on small terminals the next slide preview is dropped, and on 
very small ones only the current slide is shown. Slide transitions are not used on the master.

### Supported terminals

The new window is opened using the terminal that's detected from your environment:

* Terminal.app and iTerm2 on macOS.
* kitty (using `kitty @ launch` if remote control is enabled and a new process otherwise), WezTerm, Alacritty, Ghostty.
* GNOME Terminal and Konsole.

If the terminal can't be detected or the window can't be opened, _presenterm_ prints the command that you need to run
in another terminal and waits for it to connect:

```bash
presenterm --dual-slave 127.0.0.1:41234 -- /path/to/demo.md
```

### Customizing how the window is opened

You can set `dual.spawn_command` in your [configuration file](../configuration/introduction.md) to use any other
command. `{cmd}` gets replaced by the command that needs to be run in the new window:

```yaml
dual:
  spawn_command: "my-terminal --new-window -e {cmd}"
```

### Limitations

* The slides, notes and preview are the only things that are synchronized. Hot reloading and code execution output 
are handled independently by each instance.
* There can only be one slave per master.
