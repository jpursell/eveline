# eveline

A polargraph made out of scraps and 3D printed parts.

## Description

The main (and only) motors are at the top left and right corners of a wooden
plank that is 298 mm wide by 392 mm tall

The motors are japan servo Type KP4M4-029 unipolar steppers.
They have 100 step per revolution and might be designed for 12 volts. I was
unable to find a data sheet. They are NEMA 17. It has a 5 mm round shaft. The
black wire is VCC and the wire order is 1,4,2,3 for stepping. At 5 volts it
draws about 50 mA (250 mW). At 12 volts it should be around 120 mA and 1.4 W
with one coil energised. The max RPM is around 200.
The motors do not have any internal gearing. I can turn them while energised by
twisting the spindle with my hand, overcomming the holding torque, but it seems
like enough for the plotter.

I'm driving the motors with the cheap board that comes with a 28BYJ-48 5V stepper motor.

I attached a spindle to the motor with an inner diameter of 5 mm to fit the
motor shaft and a diameter of 19.6 mm, radius 9.8 mm, circumference 61.5 mm.
Whole steps will move it 0.6 mm and half steps will be 0.3 mm. I have found
microstepping to be problematic. The top speed is around 200 mm per second which
would clear the vertical distance in about 2 seconds.

## Challanges

I was unable to get microstepping working with the driver I have. For one thing,
the algorithm I was using seems to be wrong because the stpe distance does not
seem equal at all. I found a paper on microstepping at
https://cdn.weka-fachmedien.de/whitepaper/files/109_mca486wpmicrostepper_algorithm_01.pdf#:~:text=*%20Equation%205:%20*%2090%C2%B0/8%20=%2011.25%C2%B0/step.,1/8%20Microstepping%20to%20Complete%202%20Full%20Steps.
that seems to indicate using sin(angle) to modulate the power of the secondary
coil. I was using the max torque version of the algorithm which only requires a
single PWM. One thing that was worrysome is that the stepper driver board has
lights to indicate which channels are on and I was seeing another channel light
up when the PWM was on. I don't have an oscilloscope, but I'm guessing the issue
is that the inductive load of the motor is overwhelming the freewheeling diodes
in the stepper driver when it's switched on and off that fast.

The movement is
very jerky with whole stepping and somewhat jerky with half stepping. If
microstepping is not going to work (It's also very noisy), then perhaps I could
3D print a gear reduction. If I did that, I could use the more straight forward
(and max torque) method of whole stepping with 2 coils energized. I don't know
what the best ratio would be but a 12 to 1 reduction would get me down to 20
steps per mm (0.05 mm per step) which I think would be good enough. That would
result in it taking 12 times longer to move across the board so 24 seconds
(16.7 mm per second). I think maybe 10 mm per second would be a good speed so
this seems ok as long as we are not having to jog across the board to start a
new line a lot. Optimization can help with that. I was wondering if using a
hilbert curve to sort the line starting positions would work well, or perhaps
using a greedy min distance from ending position to starting position.

## Planetary Gear Notes

R = 2P + S
P = (R - S) / 2

where 
R is the number of teeth in the ring
P is the number of teeth in the planets
S is the number of teeth in the sun

(R + S)Ty = RTr + TsS

where
Tr is the turns of the ring gear
Ts is the turns of the sun gear
Ty is the turns of the planetary carrier

Assuming ring is stationary we get
(R + S)Ty = TsS
Ty = TsS/(R + S)
Ty/Ts=S/(R+S)

let OR = Ty/Ts and solve for R
(R+S)OR = S
ROR + SOR = S
ROR = S - SOR
R = (S - SOR) / OR

If we assumed we had 8 teeth on the sun and wanted a 1 to 12 ratio
R = (8 - 8/12) * 12 = 88
P = (R - S) / 2
  = (88 - 8) / 2 = 40

so if we had a 13 mm sun gear we would have 80 mm planet gears and 
a 140 mm ring gear which is pretty big.

You could also do 2 stages and make it taller

if you had (assume 5 mm per tooth)
R = 20 (32 mm)
S = 8 (13 mm)
P = 6 (10 mm)
OR = 1:12.25 for 2 stages

### Gear Parameters

From the internet
double helix gears are good
25 degree pressure angle is recomended
Looks like 0.3 mm module is too small

Module = OD / (2 + T)
so if I wanted to have 20 teeth and 32 mm OD
Module = 32 / (2 + 20) = 1.45 ish. 
It looks like 1.5 is a standard module so we can use that
other params:
gear height: 10 mm
helix angle: 15 deg

### Simple 1:12 2 Gear Plan
small gear: 9 teeth
large gear: 108 teeth (this is too big)

### Double reduction 1:11.8
small gear: 9 teeth
large gear: 31 teeth