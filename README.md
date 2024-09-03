A basic prompt written in rust

```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

```bash
if [ -f "$HOME"/projects/powerline-rust/target/release/powerline-rust ]; then
  function _update_ps1() {
    eval $("$HOME"/projects/powerline-rust/target/release/powerline-rust $?)
  }
  if [ "$TERM" != "linux" ]; then
    PROMPT_COMMAND="_update_ps1; $PROMPT_COMMAND"
  fi
fi
```

![screenshot](scrn.png)
