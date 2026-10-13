cmake_minimum_required(VERSION 3.20)
project(MushaCSDK NONE)
find_package(Python3 REQUIRED COMPONENTS Interpreter)
get_filename_component(MUSHA_SDK "${CMAKE_CURRENT_SOURCE_DIR}" ABSOLUTE)
if(EXISTS "${MUSHA_SDK}/tools/check-sdk.py")
  set(MUSHA_CHECK "${MUSHA_SDK}/tools/check-sdk.py")
else()
  set(MUSHA_CHECK "${MUSHA_SDK}/../tools/check-sdk.py")
endif()
# Run the canonical freestanding compile/link recipe; never execute the guest.
add_custom_target(hello ALL
  COMMAND "${Python3_EXECUTABLE}" "${MUSHA_CHECK}" --c-only --sdk "${MUSHA_SDK}" --output "${CMAKE_BINARY_DIR}/checked"
  BYPRODUCTS "${CMAKE_BINARY_DIR}/checked/hello-c.elf"
  VERBATIM)
