# WPass: What is the password?

`WPass` is a simple library that makes calls to `7z` to try to extract a password-protected archive. You provide a possible dict file for `WPass`, `WPass` tests each password in the dict file against the archive. If the password is correct, `WPass` will extract the archive to the specified directory.

## Why `WPass`?

This is meant to be the minimal interface on top of `7z`. It is meant to be small, simple but easy to use. Mainly it is implemented for my own convenience, but I hope it can be useful to others as well.

## Extra interface to `WPass`

Of course a mere rust library is of no use at all. This repository contains a small cli wrapper on `WPass`. If you want a GUI you can go to [`WPass-gui`](https://github.com/asternight/wpass-gui).

## Planned features

- Find all volumes for one archive

## Some design choices

> Why not just use `7z` directly?

Of course, it is possible, to combine 7z and some bash scripting. But it prevents further development. I'm not sure how to wrap GUI around bash scripts.

> OK, then why don't you get away from `7z` and use something like `libarchive` or `compress-tools`?

`libarchive` and `compress-tools` are both cool. But since I'm mainly using `WPass` on Windows I found these libraries rather hard to use. You can see in the code there are quite some windows specific code. While I'm trying to get rid of these code I'm not really motivated to do so. So the library stuck in a delicate balance between bash script and `libarchive`. (You don't really run bash script on Windows, do you?)
