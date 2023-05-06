# boxxy

boxxy is a tool for boxing up misbehaving Linux applications and forcing them
to put their files and directories in the right place, **without symlinks!**

boxxy is a part of the [amyware discord server](https://discord.gg/7WgSTwh).

Linux-only! boxxy uses Linux namespaces for its functionality.

For example, consider tmux. It wants to put its config in `~/.tmux.conf`. With
boxxy, you can put its config in `~/.config/tmux/tmux.conf` instead:

```yaml
# ~/.config/boxxy/boxxy.yaml
rules:
- name: "redirect tmux config from ~/.tmux.conf to ~/.config/tmux/tmux.conf"
  target: "~/.tmux.conf"
  rewrite: "~/.config/tmux/tmux.conf"
  mode: "file"
```

[![asciicast](https://asciinema.org/a/558679.svg)](https://asciinema.org/a/558679)

## motivation

I recently had to use the AWS CLI. It wants to save data in `~/.aws`, but I
don't want it to just clutter up my `$HOME` however it wants. boxxy lets me
force it to puts its data somewhere nice and proper.

## features

- box any program and force it to put its files/directories where you want it to
- context-dependent boxing, ie different rules apply in different directories
  depending on your configuration
- minimal overhead
- opt-in immutable fs outside of rule rewrites, ie only the files/directories
  you specify in rules are writable
- `0.5.0`: boxxy can scan your homedir to automatically suggest rules for
  you! ![image of boxxy scan](https://cdn.mewna.xyz/2023/03/25/G6hrd3iQjEy65.png)
- `0.6.0`: boxxy can use project-local `boxxy.yaml` files, and can load
  `.env` files for you! ![image of 0.6.0 features](https://cdn.mewna.xyz/2023/03/28/Jawp5It1xrnWN.png)
- `0.6.1`: boxxy rules can inject env vars: ![image of 0.6.1 features](https://cdn.mewna.xyz/2023/03/29/ukcWuiYdtI8yq.png)

### potential drawbacks

- new project, 0.x.y, comes with all those warnings
- **cannot** use sudo inside the container (see [#6](https://github.com/queer/boxxy/issues/6))
- primarily tested for my use-cases

## example usage

```sh
git:(mistress) | ▶  cat ~/.config/boxxy/boxxy.yaml
rules:
- name: "Store AWS CLI config in ~/.config/aws"
  target: "~/.aws"
  rewrite: "~/.config/aws"

git:(mistress) | ▶  boxxy aws configure
 INFO  boxxy > loaded 1 rules
 INFO  boxxy::enclosure > applying rule 'Store AWS CLI config in ~/.config/aws'
 INFO  boxxy::enclosure > redirect: ~/.aws -> ~/.config/aws
 INFO  boxxy::enclosure > boxed "aws" ♥
AWS Access Key ID [****************d]: a
AWS Secret Access Key [****************c]: b
Default region name [b]: c
Default output format [a]: d
git:(mistress) | ▶  ls ~/.aws
git:(mistress) | ▶  ls ~/.config/aws
config  credentials
git:(mistress) | ▶  cat ~/.config/aws/config
[default]
region = c
output = d
git:(mistress) | ▶
```

### suggested usage

- `alias aws="boxxy aws"` (repeat for other tools)
- use contexts to keep project configs separate on disk
- dotfiles!
- stop using symlinks!!!
- no more dev config files when writing code

## configuration

The boxxy configuration file lives in `~/.config/boxxy/boxxy.yaml`. If none
exists, an empty one will be created for you.

```yaml
rules:
# The name of the rule. User-friendly name for your reference
- name: "redirect aws-cli from ~/.aws to ~/.config/aws"
  # The target of the rule, ie the file/directory that will be shadowed by the
  # rewrite.
  target: "~/.aws"
  # The rewrite of the rule, ie the file/directory that will be used instead of
  # the target.
  rewrite: "~/.config/aws"
- name: "use different k8s configs when in ~/Projects/my-cool-startup"
  target: "~/.kube/config"
  rewrite: "~/Projects/my-cool-startup/.kube/config"
  # The context for the rule. Any paths listed in the context are paths where
  # this rule will apply. If no context is specified, the rule applies
  # globally.
  context:
  - "~/Projects/my-cool-startup"
  # The mode of this rule, either `directory` or `file`. `directory` is the
  # default. Must be specified for the correct behaviour when the target is a
  # file. Required because the target file/directory may not exist yet.
  mode: "file"
  # The list of commands that this rule applies to. If no commands are
  # specified, the rule applies to all programs run with boxxy.
  only:
  - "kubectl"
```

### syntax

```yaml
rules:
- name: "any valid string" # required
  target: "path" # required
  rewrite: "path" # required
  context: # optional
  - "path"
  - "path"
  mode: "directory | file" # optional
  only: # optional
  - "binary name"
  - "binary name"
  env: # optional
    KEY: "value"
```

## developing

1. set up pre-commit: `pre-commit install`
2. make sure it builds: `cargo build`
3. do the thing!
4. test with the command of your choice, ex. `cargo run -- ls -lah ~/.config`

### how does it work?

- create temporary directory in /tmp
- set up new user/mount namespace
- bind-mount `/` to tmp directory
- bind-mount rule mounts rw so that target programs can use them
- remount `/` ro
- run!
