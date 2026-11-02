presenterm
===

[![crates-badge]][crates-package] [![brew-badge]][brew-package] [![nix-badge]][nix-package] 
[![arch-badge]][arch-package] [![scoop-badge]][scoop-package] [![winget-badge]][winget-package]

[brew-badge]: https://img.shields.io/homebrew/v/presenterm
[brew-package]: https://formulae.brew.sh/formula/presenterm
[nix-badge]: https://img.shields.io/badge/Packaged_for-Nix-5277C3.svg?logo=nixos&labelColor=73C3D5
[nix-package]: https://search.nixos.org/packages?size=1&show=presenterm
[crates-badge]: https://img.shields.io/crates/v/presenterm
[crates-package]: https://crates.io/crates/presenterm
[arch-badge]: https://img.shields.io/archlinux/v/extra/x86_64/presenterm
[arch-package]: https://archlinux.org/packages/extra/x86_64/presenterm/
[scoop-badge]: https://img.shields.io/scoop/v/presenterm
[scoop-package]: https://scoop.sh/#/apps?q=presenterm&id=a462290f824b50f180afbaa6d8c7c1e6e0952e3a
[winget-badge]: https://img.shields.io/winget/v/mfontanini.presenterm
[winget-package]: https://winstall.app/apps/mfontanini.presenterm

_presenterm_ lets you create presentations in markdown format and run them from your terminal, with support for image 
and animated gifs, highly customizable themes, code highlighting, exporting presentations into PDF format, and plenty of 
other features. This is how the [demo presentation](/examples/demo.md) looks like when running in the [kitty 
terminal](https://sw.kovidgoyal.net/kitty/):

![](/docs/src/assets/demo.gif)

Check the rest of the example presentations in the [examples directory](/examples).

# Documentation

Visit the [documentation][docs-introduction] to get started.

# Features

* Presentations consist of one [or more][docs-include] markdown files.
* [Images and animated gifs][docs-images] on terminals like _kitty_, _iterm2_, _wezterm_, _ghostty_ and _foot_.
* [Customizable themes][docs-themes] including colors, margins, layout (left/center aligned content), footer for every 
  slide, etc. Several [built-in themes][docs-builtin-themes] can give your presentation the look you want without 
  having to define your own.
* [Centered slides and images][docs-centering]: start every slide centered vertically and horizontally with 
  `center_slides`, or center individual slides, paragraphs and images.
* Code highlighting for a [wide list of programming languages][docs-code-highlight].
* [Font sizes][docs-font-sizes] for terminals that support them.
* [Selective/dynamic][docs-selective-highlight] code highlighting that only highlights portions of code at a time.
* [Column layouts][docs-layout].
* [mermaid graph rendering][docs-mermaid].
* [d2 graph rendering][docs-d2].
* [_LaTeX_ and _typst_ formula rendering][docs-latex].
* [Introduction slide][docs-intro-slide] that displays the presentation title and your name.
* [Slide titles][docs-slide-titles].
* [Snippet execution][docs-code-execute] for various programming languages, including execution inside pseudo terminals.
* [Export presentations to PDF and HTML][docs-exports].
* [Slide transitions][docs-slide-transitions].
* [Pause][docs-pauses] portions of your slides.
* [Custom key bindings][docs-key-bindings].
* [Automatically reload your presentation][docs-hot-reload] every time it changes for a fast development loop.
* [Define speaker notes][docs-speaker-notes] to aid you during presentations.
* [Present from two terminals][docs-dual] with a master view showing notes and the next slide.

See the [introduction page][docs-introduction] to learn more.

# Getting started

Install _presenterm_ using one of the methods in the [installation guide][docs-install], then start a presentation with:

```bash
presenterm path/to/presentation.md
```

To try the included demo, clone the repository and run:

```bash
git clone https://github.com/VanBelleKoen/presenterm.git
cd presenterm
presenterm examples/demo.md
```

If you have Rust installed, you can install _presenterm_ directly with Cargo:

```bash
cargo install --locked presenterm
```

## Command-line reference

The general form is `presenterm [OPTIONS] [PATH]`. `PATH` is the Markdown file containing the presentation. Run
`presenterm --help` for the same options in your terminal.

### Starting and customizing a presentation

* `-p`, `--present`: start in presentation mode. By default, _presenterm_ uses development mode.
* `-t NAME`, `--theme NAME`: use the named theme instead of the configured default.
* `--image-protocol PROTOCOL`: select how terminal images are displayed. Supported values are `auto` (detect the
  terminal's best option), `iterm2`, `iterm2-multipart`, `kitty-local`, `kitty-remote`, `sixel`, and `ascii-blocks`.
* `-c FILE`, `--config-file FILE`: load settings from `FILE` rather than the default configuration file. The
  `PRESENTERM_CONFIG_FILE` environment variable can be used instead.
* `-x`, `--enable-snippet-execution`: enable execution of code snippets in the presentation.
* `-X`, `--enable-snippet-execution-replace`: enable automatic execution and replacement for `+exec_replace` code
  blocks.
* `--validate-overflows`: check whether presentation content overflows the terminal screen.
* `--validate-snippets`: validate code snippets in the presentation.

### Exporting

* `-e`, `--export-pdf`: export the presentation to PDF instead of displaying it. PDF export requires
  [WeasyPrint][docs-exports].
* `-E`, `--export-html`: export the presentation to a self-contained HTML file.
* `-o FILE`, `--output FILE`: write the exported file to `FILE` instead of the default path next to the presentation.
* `--export-temporary-path DIR`: store temporary export files in `DIR` instead of a temporary directory.

See the [export guide][docs-exports] for more details.

### Themes and information

* `--list-themes`: browse the available themes.
* `--current-theme`: print the name of the theme currently in use.
* `--list-comment-commands`: list the available Markdown comment commands.
* `--acknowledgements`: display acknowledgements.
* `-h`, `--help`: display command-line help.
* `-V`, `--version`: display the installed version.

When built with the optional Cargo `json-schema` feature, `--generate-config-file-schema` prints the configuration
file's JSON schema.

### Speaker notes and dual-terminal mode

* `-d`, `--dual`: open a second terminal window showing the current slide, while this terminal shows speaker notes and
  a preview of the next slide.

Read more about [speaker notes][docs-speaker-notes] and [dual-terminal mode][docs-dual].

# presenterm in action

Here are some talks and demos that feature _presenterm_:

- [If it can compute, it can run Ratatui][it-can-run-ratatui] by [Orhun Parmaksız][orhun-github]
- [Building Embedded TUIs with Rust & Ratatui][building-embedded-tuis] by [Orhun Parmaksız][orhun-github]
- [Bringing Terminal Aesthetics to the Web With Rust][bringing-terminal-aesthetics] by [Orhun Parmaksız][orhun-github]
- [7 Rust Terminal Tools That You Should Use][rust-terminal-tools] by [Orhun Parmaksız][orhun-github]
- [Renaissance of Terminal User Interfaces with Rust][renaissance-tui] by [Orhun Parmaksız][orhun-github]
- [Using Nix on Apple Silicon and declarative development environments][NiXOS-and-Dev] by [pwnwriter][pwnwriter-github]
- [Hayasen: A Robust Embedded Rust Library which supports multiple sensors][hayasen] by [Vaishnav-Sabari-Girish][vaishnav]
- [Using ratatui in Embedded sytems : Meet Mousefood][mousefood] by [Vaishnav-Sabari-Girish][vaishnav]

Gave a talk using _presenterm_? We would love to feature it here! Open a PR or issue to get it added.

<!-- links -->

[docs-introduction]: docs/src/introduction.md
[docs-install]: docs/src/install.md
[docs-basics]: docs/src/features/introduction.md
[docs-intro-slide]: docs/src/features/introduction.md#introduction-slide
[docs-slide-titles]: docs/src/features/introduction.md#slide-titles
[docs-font-sizes]: docs/src/features/introduction.md#font-sizes
[docs-pauses]: docs/src/features/commands.md#pauses
[docs-images]: docs/src/features/images.md
[docs-include]: docs/src/features/commands.md#including-external-markdown-files
[docs-themes]: docs/src/features/themes/introduction.md
[docs-builtin-themes]: docs/src/features/themes/introduction.md#built-in-themes
[docs-code-highlight]: docs/src/features/code/highlighting.md
[docs-code-execute]: docs/src/features/code/execution.md
[docs-selective-highlight]: docs/src/features/code/highlighting.md#selective-highlighting
[docs-slide-transitions]: docs/src/features/slide-transitions.md
[docs-layout]: docs/src/features/layout.md
[docs-mermaid]: docs/src/features/code/mermaid.md
[docs-d2]: docs/src/features/code/d2.md
[docs-latex]: docs/src/features/code/latex.md
[docs-exports]: docs/src/features/exports.md
[docs-key-bindings]: docs/src/configuration/settings.md#key-bindings
[docs-hot-reload]: docs/src/features/introduction.md#hot-reload
[docs-speaker-notes]: docs/src/features/speaker-notes.md
[docs-dual]: docs/src/features/dual-terminal.md
[docs-centering]: docs/src/features/commands.md#centering-a-whole-presentation
[bat]: https://github.com/sharkdp/bat
[syntect]: https://github.com/trishume/syntect
[bringing-terminal-aesthetics]: https://www.youtube.com/watch?v=iepbyYrF_YQ
[building-embedded-tuis]: https://www.youtube.com/watch?v=F04kQMKwrwQ
[it-can-run-ratatui]: https://www.youtube.com/watch?v=btqNDDuZ3cI
[rust-terminal-tools]: https://www.youtube.com/watch?v=ATiKwUiqnAU
[renaissance-tui]: https://www.youtube.com/watch?v=hWG51Mc1DlM
[orhun-github]: https://github.com/orhun
[NiXOS-and-Dev]: https://github.com/pwnwriter/PTN11
[pwnwriter-github]: https://github.com/pwnwriter
[hayasen]: https://github.com/Vaishnav-Sabari-Girish/rust_bangalore_oct_2025
[vaishnav]: https://github.com/Vaishnav-Sabari-Girish
[mousefood]: https://github.com/Vaishnav-Sabari-Girish/rust_bangalore_december_2025/
