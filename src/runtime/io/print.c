#include "../string/common.h"
#include <stdio.h>

void print(WillowString *str)
{
    if (str == NULL)
    {
        printf("null\n");
        return;
    }
    fwrite(str->data, 1, str->len, stdout);
    printf("\n");
}
