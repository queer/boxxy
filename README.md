# boxxy

boxxy is a tool for boxing up misbehaving Linux applications and forcing them
to put their files and directories in the right place, **without symlinks!**

Linux-only! boxxy uses Linux namespaces for its functionality.

## motivation

I recently had to use the AWS CLI. It wants to save data in `~/.aws`, but I
don't want it to just clutter up my `$HOME` however it wants. boxxy lets me
force it to puts its data somewhere nice and proper.

## features

- box any program and force it to put its files/directories where you want it to
- context-dependent boxing, ie different rules apply in different directories
  depending on your configuration
- minimal overhead
- opt-in immutable fs outside of rule rewrites

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

- `alias aws="boxxy aws"` (repeat for other clouds)
- use contexts to keep project configs separate on disk
- stop using symlinks!!!

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
