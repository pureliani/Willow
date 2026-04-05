#ifndef WILLOW_STRING_H
#define WILLOW_STRING_H

#include <stdint.h>
#include <stddef.h>

typedef struct
{
    size_t len;
    char data[];
} WillowString;

#endif
