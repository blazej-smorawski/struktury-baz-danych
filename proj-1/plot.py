#!/bin/python3

import numpy as np
import matplotlib.pyplot as plt
import math
from matplotlib import rcParams

rcParams['font.family'] = 'serif'
rcParams['font.sans-serif'] = ['Computer Modern']

data = np.genfromtxt('measurements.csv', delimiter='\t')

#b = 240/64
b = 3
xs = data[:,3]
runs = data[:,0]
runs_t = [math.log2(N) for N in xs]
read = data[:,1]
write = data[:,1]
rw_t = [4*N * math.ceil(math.log2(N))/b for N in xs]

fig1, ax1 = plt.subplots()

ax1.plot(xs, runs, label="Zmierzona liczba przebiegów")
ax1.plot(xs, runs_t, label="Teoretyczna maksymalna liczba przebiegów")
ax1.axis('equal')
ax1.set_xscale("log", base=2) 
ax1.grid()
ax1.autoscale()
ax1.set_title("Liczba przebiegów sortowania w zależności od N")
ax1.set_ylabel("Liczba przebiegów")
ax1.set_xlabel("N")
ax1.legend()
fig1.savefig('runs.png', dpi=300)       

fig2, ax2 = plt.subplots()
print(write)
ax2.plot(xs, read, label="Zmierzone odczyty")
ax2.plot(xs, write, label="Zmierzone Zapisy")
ax2.plot(xs, rw_t, label="Teoretyczna maksymalna liczba operacji")
ax2.set_xscale("log", base=2) 
ax2.set_yscale("log", base=2) 
ax2.grid()
ax2.set_title("Operacje IO w zależności od N")
ax2.set_ylabel("Operacje IO")
ax2.set_xlabel("N")
ax2.legend()
fig2.savefig('io.png', dpi=300)       
