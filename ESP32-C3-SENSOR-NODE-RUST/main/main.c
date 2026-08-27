#include <stdio.h>

/* Được cung cấp bởi component `rust_sensor_node`
(xem components/rust_sensor_node/src/lib.rs). */
extern int rust_sensor_node_main(void);

void app_main(void) {
    printf("HYDRAGROW Sensor Node: khoi dong C main, ban giao cho Rust...\n");
    int result = rust_sensor_node_main();
    /* Trong vong lap binh thuong, rust_sensor_node_main() khong bao gio return.
    Neu return, nghia la co loi khoi tao nghiem trong o phia Rust. */
    printf("rust_sensor_node_main() da thoat voi ma loi: %d\n", result);
}
