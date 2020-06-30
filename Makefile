.PHONY: compile

compile:
	cargo build
run:
	./target/debug/eventuler
clean:
	rm -r ./target/debug
