# Cac component ESP-IDF can bat de cac wrapper Rust (esp-idf-hal/esp-idf-svc)
# hoat dong day du: WiFi, MQTT, HTTP client, NVS, driver I2C/GPIO...
set(RUST_DEPS "pthread" "driver" "vfs" "console" "nvs_flash" "spi_flash"
              "mqtt" "esp_http_client" "esp_driver_gpio" "esp_driver_i2c"
              "esp_driver_gptimer" "esp_driver_uart")

idf_component_register(
    SRCS "placeholder.c"
    INCLUDE_DIRS ""
    PRIV_REQUIRES "${RUST_DEPS}" "sensor_ffi"
)

if(CONFIG_IDF_TARGET_ESP32C3)
    set(RUST_TARGET "riscv32imc-esp-espidf")
else()
    message(FATAL_ERROR "components/rust_sensor_node chi ho tro ESP32-C3 hien tai (${CONFIG_IDF_TARGET})")
endif()

if(CMAKE_BUILD_TYPE STREQUAL "Debug")
    set(CARGO_BUILD_TYPE "debug")
    set(CARGO_BUILD_ARG "")
else()
    set(CARGO_BUILD_TYPE "release")
    set(CARGO_BUILD_ARG "--release")
endif()

set(CARGO_BUILD_STD_ARG -Zbuild-std=std,panic_abort)

if(IDF_VERSION_MAJOR GREATER "4")
    set(ESP_RUSTFLAGS "--cfg espidf_time64")
endif()

set(CARGO_PROJECT_DIR "${CMAKE_CURRENT_LIST_DIR}")
set(CARGO_BUILD_DIR "${CMAKE_CURRENT_BINARY_DIR}")
set(CARGO_TARGET_DIR "${CARGO_BUILD_DIR}/target")

set(RUST_INCLUDE_DIR "${CARGO_TARGET_DIR}")
set(RUST_STATIC_LIBRARY "${CARGO_TARGET_DIR}/${RUST_TARGET}/${CARGO_BUILD_TYPE}/libesp32_c3_sensor_node.a")

idf_build_get_property(sdkconfig SDKCONFIG)
idf_build_get_property(idf_path IDF_PATH)

ExternalProject_Add(
    project_rust_sensor_node
    PREFIX "${CARGO_PROJECT_DIR}"
    DOWNLOAD_COMMAND ""
    CONFIGURE_COMMAND ${CMAKE_COMMAND} -E env
    cargo clean --target ${RUST_TARGET} --target-dir ${CARGO_TARGET_DIR}
    USES_TERMINAL_BUILD true
    BUILD_COMMAND ${CMAKE_COMMAND} -E env
        CARGO_CMAKE_BUILD_INCLUDES=$<TARGET_PROPERTY:${COMPONENT_LIB},INCLUDE_DIRECTORIES>
        CARGO_CMAKE_BUILD_LINK_LIBRARIES=$<TARGET_PROPERTY:${COMPONENT_LIB},LINK_LIBRARIES>
        CARGO_CMAKE_BUILD_SDKCONFIG=${sdkconfig}
        CARGO_CMAKE_BUILD_ESP_IDF=${idf_path}
        CARGO_CMAKE_BUILD_COMPILER=${CMAKE_C_COMPILER}
        RUSTFLAGS=${ESP_RUSTFLAGS}
        MCU=${CONFIG_IDF_TARGET}
        cargo build --target ${RUST_TARGET} --target-dir ${CARGO_TARGET_DIR} ${CARGO_BUILD_ARG} ${CARGO_BUILD_STD_ARG}
    INSTALL_COMMAND ""
    BUILD_ALWAYS TRUE
    TMP_DIR "${CARGO_BUILD_DIR}/tmp"
    STAMP_DIR "${CARGO_BUILD_DIR}/stamp"
    DOWNLOAD_DIR "${CARGO_BUILD_DIR}"
    SOURCE_DIR "${CARGO_PROJECT_DIR}"
    BINARY_DIR "${CARGO_PROJECT_DIR}"
    INSTALL_DIR "${CARGO_BUILD_DIR}"
    BUILD_BYPRODUCTS "${RUST_STATIC_LIBRARY}")

add_prebuilt_library(library_rust_sensor_node "${RUST_STATIC_LIBRARY}" PRIV_REQUIRES "${RUST_DEPS}" "sensor_ffi")
add_dependencies(library_rust_sensor_node project_rust_sensor_node)

target_include_directories(${COMPONENT_LIB} PUBLIC "${RUST_INCLUDE_DIR}")
target_link_libraries(${COMPONENT_LIB} PRIVATE library_rust_sensor_node)
