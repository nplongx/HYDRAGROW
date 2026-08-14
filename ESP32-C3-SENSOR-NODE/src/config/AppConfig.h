#pragma once 
#include <Arduino.h> 

struct SensorConfig { 
    float phV686; 
    float phV4; 
    float phV918; 
    float tdsFactor; 
    float tdsOffset; 
    float tempOffset; 
    float tankHeight; 
    bool enablePh; 
    bool enableTds; 
    bool enableTemp; 
    bool enableWater; 
};

struct AppConfig { 
    SensorConfig sensor; 
    unsigned long publishInterval; 
    bool debugLog; 
    bool continuousLevel; 
};

extern AppConfig appConfig;