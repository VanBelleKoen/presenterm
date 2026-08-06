---
title: Center Slides Test
sub_title: Theme-level default centering
options:
  end_slide_shorthand: true
theme:
  override:
    default:
      center_slides: true
---

# Theme Centering Smoke Test

All slides in this deck should start centered vertically and use centered text alignment by default.

---

Plain paragraph line 1
Plain paragraph line 2

If center_slides is working, both lines should be horizontally centered near the middle of the screen.

---

<!-- alignment: left -->

This slide forces left alignment.
It should still begin around the vertical middle, but text should be left-aligned.

---

<!-- alignment: right -->

This slide forces right alignment.
It should still begin around the vertical middle, but text should be right-aligned.

---

<!-- jump_to_middle -->

This tests compatibility with existing commands.
With center_slides enabled, this should remain centered horizontally.

---

<!-- jump_to_center -->

This explicitly applies jump_to_center.
Behavior should match the default centered slide behavior.

---

<!-- alignment: left -->

Left override should be scoped to this slide only.

---

New slide should be centered again by default.
This confirms per-slide state reset is working.
