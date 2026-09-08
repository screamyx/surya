#!/bin/sh
# An agent that consumes stdin and never answers initialize: the
# "thinking for minutes, then nothing" startup class (issue #93). sleep
# inherits the stdio pipes and holds them open without ever replying, so
# this is a true wedge rather than a crash.
exec sleep 1000
