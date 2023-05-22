#!/usr/bin/env bash

set -eoux pipefail
cd ./forks
python -m venv forker
source forker/bin/activate
pip3 install uwsgi
uwsgi --daemonize 2
