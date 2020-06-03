# terminal-input

Cross-terminal precise decoding of modified keys and other input events. Currently being used for
[csvsheet](https://github.com/gereeter/csvsheet).

## Support table

`terminal-input` has not yet been tested on macOS and has no Windows support currently. These are
desired platforms; if you have the ability to test or help port to those (and other) operating
systems, issues and pull requests are welcome.

| | `uxterm` | `kitty` | `urxvt` | `gnome-terminal` | `alacritty`
|-| -------- | ------- | ------- | ---------------- | -----------
| Ctrl | keyboard only | yes | most letters, mouse buttons left,scroll | most letters, all mouse | most letters, all mouse
| Alt  | yes | yes | most letters, all mouse | all letters, all mouse | all letters, all mouse
| Ctrl+Alt | yes | yes | most letters, all mouse | all letters, all mouse | all letters, all mouse
| Shift | keyboard only | specials only | no | some specials only | some specials only
| Ctrl+Shift | keyboard only | keyboard only, often release only? | no, messes with input encoding | some specials only, others either capitalized or Ctrl but not both | some specials only, others captial or Ctrl but not both
| Alt+Shift | keyboard only | yes | no | mouse buttons right,scroll | scroll only
| Ctrl+Alt+Shift | keyboard only | keyboard only | no, messes with input encoding | mouse buttons right, scroll | capital or Ctrl, not both
| key releases | no | modified only | no | no | no
| key repeats | no | no (BUG?) | no | no | no
| Ctrl+Delete | yes | yes | yes | yes | yes
| Ctrl+Backspace | looks like Backspace | yes | looks like \u{8} | looks like Backspace | looks like \u{8}
| Shift+Backspace | looks like Shift+\u{8} | yes | looks like Backspace | looks like Backspace | looks like \u{8}
| Ctrl+H | yes | yes | looks like \u{8} | looks like Backspace | looks like \u{8}
| Ctrl+I | yes | yes | looks like Tab | looks like Tab | looks like Tab
| Ctrl+J | yes | yes | looks like Enter | looks like Enter | looks like Enter
| Ctrl+M | yes | yes | looks like Enter | looks like Enter | looks like Enter

## Try it out!

```
cargo run --example event_viewer
```

The `event_viewer` example allows you to see what events `terminal-input` is receiving when you
interact with your terminal. To exit, press `Ctrl+C` or `Ctrl+Q`.

## ESCDELAY

`terminal-input` currently defaults to waiting 25 milliseconds after receiving an Escape character
to distinguish between a user-entered escape character and a terminal-generated escape sequence.
This is significantly lower than the typical ncurses value of 1 second, which should improve
responsiveness at the expense of possibly failing if an escape sequence is split up and delayed.
The 25 milliseconds gives some leeway for this delay, however. To modify this value, use the
`set_escdelay` method on `InputStream`, or as a user set the `ESCDELAY` environment variable.

In the future, there may be a speculative ESCDELAY mode in which ambiguous escapes are immediately
returned to the application along with new `Checkpoint` event. If later (within some delay) input
comes in that indicates that the escape was supposed to be part of an escape sequence, then a
`Rollback` event will be emitted, and the application should go back to the state when the last
`Checkpoint` occurred. This will allow maximal responsiveness while still being reliable over slow
connections.
