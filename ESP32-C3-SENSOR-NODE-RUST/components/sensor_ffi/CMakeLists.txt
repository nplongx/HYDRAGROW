idf_component_register(
    SRCS "sensor_ffi.c"
    INCLUDE_DIRS "include"
    # Cấp quyền cho file C truy cập vào các thư viện lõi của chip
    REQUIRES driver freertos log esp_rom
)
