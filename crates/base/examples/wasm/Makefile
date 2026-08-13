.PHONY: dev build build-wasm build-web clean
build-wasm:
	./scripts/build.sh --release
build-web:
	cd www && bun install && bun run build
build: build-wasm build-web
dev:
	./scripts/build.sh
	cd www && bun install && bun run dev
clean:
	rm -rf www/dist www/src/wasm
