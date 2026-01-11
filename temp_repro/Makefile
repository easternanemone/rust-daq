CC=g++
CFLAGS=-I. -I/opt/pvcam/sdk/include -std=c++11 -pthread
LDFLAGS=-L/opt/pvcam/library/x86_64 -lpvcam -ldl -pthread

all: stall_test

stall_test: stall_test.o Common.o
	$(CC) -o stall_test stall_test.o Common.o $(LDFLAGS)

stall_test.o: stall_test.cpp Common.h
	$(CC) $(CFLAGS) -c stall_test.cpp

Common.o: Common.cpp Common.h
	$(CC) $(CFLAGS) -c Common.cpp

clean:
	rm -f *.o stall_test
