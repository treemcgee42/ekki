
dummy_renderer: dummy_renderer.h dummy_renderer.c
	clang -c -o dummy_renderer.o dummy_renderer.c -fPIC
	clang -shared -o libdummy_renderer.so dummy_renderer.o

all: dummy_renderer

