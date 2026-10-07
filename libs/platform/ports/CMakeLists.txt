add_library(ports INTERFACE)
target_include_directories(ports INTERFACE
    ${CMAKE_CURRENT_SOURCE_DIR}/include
    # The Forkpoint SDK: property and event macros that compile to nothing unless the
    # application defines FPT_ENABLE (app/CMakeLists.txt, BODY_ECU_FORKPOINT).
    ${CMAKE_CURRENT_SOURCE_DIR}/../../../third_party/forkpoint-sdk/c/include
)

add_library(port_mocks INTERFACE)
target_include_directories(port_mocks INTERFACE
    ${CMAKE_CURRENT_SOURCE_DIR}/mock
)
target_link_libraries(port_mocks INTERFACE ports GTest::gmock)
