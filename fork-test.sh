#!/usr/bin/env bash

set -eoux pipefail
cd ./forks
python -m venv forker
source forker/bin/activate
pip3 install uwsgi
cat >> forker.py <<EOF
def application(env, start_response):
    start_response('200 OK', [('Content-Type','text/html')])
    return [b"Hello World"]
EOF
uwsgi --daemonize 2 --wsgi-file forker.py --http :9090
