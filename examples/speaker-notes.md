Speaker Notes
===

`presenterm` supports speaker notes.

Add speaker notes to your presentation with the `speaker_note` comment:

```markdown
<!-- speaker_note: Your speaker note goes here. -->
```

<!-- speaker_note: This is a speaker note from slide 1. -->

Run this presentation in dual-terminal mode to view the notes in the master terminal while showing a clean slide in the
second terminal:

<!-- speaker_note: You can use multiple speaker notes within each slide and interleave them with other markdown. -->

<!-- end_slide -->

Usage
===
<!-- speaker_note: This is a speaker note from slide 2. -->

```bash
presenterm --dual examples/speaker-notes.md
```

Navigation in the master terminal is synchronized with the second terminal.

<!-- speaker_note: Isn't that cool? -->
