#include "AppConfig.h"

AppConfig appConfig = {
    {
        2650.0f, // phV686
        3555.0f, // phV4
        1750.0f, // phV918
        0.88f,   // tdsFactor
        0.0f,    // tdsOffset
        0.0f,    // tempOffset
        100.0f,  // tankHeight
        true,    // enablePh
        true,    // enableTds
        true,    // enableTemp
        true     // enableWater
    },
    5000,        // publishInterval
    true,       // debugLog
    false        // continuousLevel
};
