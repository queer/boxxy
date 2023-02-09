# boxxy

boxxy is a tool for boxing up misbehaving applications and forcing them to put
their files and directories in the right place.

## motivation

I recently had to use the AWS CLI. It wants to save data in `~/.aws`, but I
don't want it to just clutter up my `$HOME` however it wants. boxxy lets me
force it to puts its data somewhere nice and proper.

## features

- box any program and force it to put its files/directories where you want it to
- context-dependent boxing, ie different rules apply in different directories
  depending on your configuration
- minimal overhead

## example usage

```sh
git:(mistress) 1 | ▶  cat ~/.config/boxxy/boxxy.yaml
rules:
- name: "name aws cli write to ~/.config/aws"
  target: "~/.aws"
  rewrite: "~/.config/aws"
git:(mistress) 1 | ▶  boxxy aws configure
 INFO  boxxy > loaded 1 rules
 INFO  boxxy::enclosure     > applying rule 'name aws cli write to ~/.config/aws'
 INFO  boxxy::enclosure     > applied rewrite ~/.config/aws => ~/.aws ("/home/amy/.config/aws" => "/tmp/boxxy-containers/bold-surf-9356/home/amy/.aws")
AWS Access Key ID [****************a]: a
AWS Secret Access Key [****************a]: a
Default region name [a]: a
Default output format [a]: a
git:(mistress) 2 | ▶  ls ~/.aws
git:(mistress) 2 | ▶  ls ~/.config/aws
config  credentials
git:(mistress) 2 | ▶  cat ~/.config/aws/config
[default]
region = a
output = a
git:(mistress) 2 | ▶
```

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
