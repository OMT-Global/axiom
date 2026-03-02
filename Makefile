.PHONY: test interp compile vm

test:
	python -m unittest discover -v

interp:
	python -m axiom interp examples/arith.ax

compile:
	python -m axiom compile examples/arith.ax -o /tmp/arith.axb

vm:
	python -m axiom vm /tmp/arith.axb
