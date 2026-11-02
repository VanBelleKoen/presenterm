## Speaker notes

Presentations can include speaker notes using the `speaker_note` comment command. To see them while presenting, start
_presenterm_ in [dual-terminal mode](dual-terminal.md), which displays the notes in the master terminal alongside a
preview of the next slide.

### Defining speaker notes

In order to define speaker notes you can use the `speaker_note` comment command:

```markdown
Normal text

<!-- speaker_note: this is a speaker note -->

More text
```

In dual-terminal mode, the master shows "normal text", "more text", and the note. The second terminal shows only the
current slide.

### Multiline speaker notes

You can use multiline speaker notes by using the appropriate YAML syntax:

```yaml
<!-- 
speaker_note: |
  something
  something else
-->
```
