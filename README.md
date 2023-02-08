# boxxy

boxxy is a tool for boxing up misbehaving applications and forcing them to put
their files and directories in the right place.

## Example usage

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