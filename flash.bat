@echo off

cargo objcopy --release -- -O binary nuc-091.bin

set ST_LINK="C:\Program Files\STMicroelectronics\STM32Cube\STM32CubeProgrammer\bin\STM32_Programmer_CLI.exe"

%ST_LINK% -c port=swd -w %~dp0nuc-091.bin 0x08000000

%ST_LINK% -c port=swd -rst
