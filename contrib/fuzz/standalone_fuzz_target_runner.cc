// Copyright 2017 Google Inc. All Rights Reserved.
// Licensed under the Apache License, Version 2.0 (the "License");

// Example of a standalone runner for "fuzz targets".
// It reads all files passed as parameters and feeds their contents
// one by one into the fuzz target (LLVMFuzzerTestOneInput).
// This runner does not do any fuzzing, but allows us to run the fuzz target
// on the test corpus (e.g. "do_stuff_test_data") or on a single file,
// e.g. the one that comes from a bug report.

#include <cassert>
#include <fstream>
#include <iostream>
#include <vector>

// Forward declare the "fuzz target" interface.
// We deliberately keep this inteface simple and header-free.
extern "C" int LLVMFuzzerTestOneInput(const uint8_t *data, size_t size);

extern "C" int LLVMFuzzerInitialize(int *argc, char ***argv);

int main(int argc, char **argv)
{
	LLVMFuzzerInitialize(&argc, &argv);

	for (int i = 1; i < argc; i++) {
		std::ifstream in(argv[i]);
		in.seekg(0, in.end);
		size_t length = in.tellg();
		in.seekg(0, in.beg);
		std::cout << "Reading " << length << " bytes from " << argv[i]
		          << std::endl;
		// Allocate exactly length bytes so that we reliably catch
		// buffer overflows.
		std::vector<char> bytes(length);
		in.read(bytes.data(), bytes.size());
		assert(in);
		LLVMFuzzerTestOneInput(
		    reinterpret_cast<const uint8_t *>(bytes.data()),
		    bytes.size());
		std::cout << "Execution successful" << std::endl;
	}
	return 0;
}
// no-check-code since this is from a third party
