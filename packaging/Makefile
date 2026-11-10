PROJECT := morse-runner-builder
SHELL := /bin/bash

.PHONY: build run clean distrib

all: build

build:
	docker build  --progress=plain . -f Dockerfile -t $(PROJECT)
	docker tag $(PROJECT) $(PROJECT):latest


#test:
#	# run & test the code backed into the container
#	docker run --entrypoint=/bin/bash -w /app -ti $(PROJECT):latest ".tests/run.sh"
#
#localtest:
#	# run tests on the local code
#	docker run --entrypoint=/bin/bash -v $(PWD):/app -w /app -ti $(PROJECT):latest ".tests/run.sh"
#
#run:
#	docker run $(PROJECT)
#
#build-nc:
#	docker build . -f Dockerfile -t $(PROJECT) --no-cache --pull
#	docker tag $(PROJECT) $(PROJECT):latest
#
#shell:
#	docker run --entrypoint=/bin/bash -ti $(PROJECT):latest
#
#localshell:
#	# start a shell & bind the local dir as /app
#	docker run --entrypoint=/bin/bash -v $(PWD):/app -ti $(PROJECT):latest
