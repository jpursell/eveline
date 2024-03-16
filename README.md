# eveline

A polargraph made out of scraps and 3D printed parts.

## Description

The main (and only) motors are at the top left and right corners of a wooden
plank that is 298 mm wide by 392 mm tall

The motors are japan servo Type KP4M4-029 5-wire unipolar steppers.
They have 100 step per revolution and might be designed for 12 volts. I was
unable to find a data sheet. They are NEMA 17. It has a 5 mm round shaft. The
black wire is VCC and the wire order is 1,4,2,3 for stepping. At 5 volts it
draws about 50 mA (250 mW). At 12 volts it should be around 120 mA and 1.4 W
with one coil energised. The max RPM is around 200.
The motors do not have any internal gearing. I can turn them while energised by
twisting the spindle with my hand, overcomming the holding torque, but it seems
like enough for the plotter.

I'm driving the motors with the cheap board that comes with a 28BYJ-48 5V stepper motor.

Microstepping did not work. I think I would need different hardware.
I designed a 12:1 compound gear mechanism to reduce the step side down drastically.

## Gear Parameters

module: 0.7 mm
pressure angle: 25 degrees
pinion teeth: 17
big gear teeth: 59
backlash: 0.2 mm
spool minimum radius: 5.75 mm
actual gear reduction: (59/17)^2 aprox_eq 12.045

Module = OD / (2 + T)

### Printing

I'm using orcaslicer and polyholes. I use compliant grippers for holding the bearings and
attaching the pinion to the motor shaft.