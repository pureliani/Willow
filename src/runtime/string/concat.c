#include "common.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

// The intrinsic called by the MIR Builder for `${x} ${y}`
WillowString *concat_strings(WillowString **parts, size_t count)
{
    size_t total_len = 0;
    for (size_t i = 0; i < count; i++)
    {
        if (parts[i] != NULL)
        {
            total_len += parts[i]->len;
        }
    }

    WillowString *result = (WillowString *)malloc(sizeof(WillowString) + total_len);
    if (result == NULL)
    {
        fprintf(stderr, "Out of memory during string concatenation\n");
        exit(1);
    }

    result->len = total_len;

    size_t current_offset = 0;
    for (size_t i = 0; i < count; i++)
    {
        if (parts[i] != NULL && parts[i]->len > 0)
        {
            memcpy(result->data + current_offset, parts[i]->data, parts[i]->len);
            current_offset += parts[i]->len;
        }
    }

    return result;
}
