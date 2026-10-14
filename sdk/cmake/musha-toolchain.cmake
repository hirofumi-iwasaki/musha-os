# Experimental C cross-build. Explicit MUSHA_SDK, Clang and LLD paths required.
set(CMAKE_SYSTEM_NAME Generic)
set(CMAKE_SYSTEM_PROCESSOR x86_64)
set(CMAKE_TRY_COMPILE_TARGET_TYPE STATIC_LIBRARY)
set(CMAKE_C_COMPILER_TARGET x86_64-unknown-none-elf)
set(CMAKE_SYSROOT "${MUSHA_SDK}/sysroot")
set(CMAKE_C_FLAGS_INIT "-ffreestanding -fno-stack-protector -fno-pic -mno-red-zone -march=x86-64 -mno-avx")
# Canonical checked build is tools/check-sdk.py; no host libc/crt is linked.
