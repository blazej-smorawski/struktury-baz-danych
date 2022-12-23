#!/bin/python3

import numpy as np
import matplotlib.pyplot as plt
import math
from matplotlib import rcParams

rcParams['font.family'] = 'serif'
rcParams['font.sans-serif'] = ['Computer Modern']

# ------------- insert -------------------

insert2 = np.genfromtxt('insert-result-84.txt', delimiter=' ')
insert3 = np.genfromtxt('insert-result-126.txt', delimiter=' ')
insert4 = np.genfromtxt('insert-result-168.txt', delimiter=' ')
insert5 = np.genfromtxt('insert-result-210.txt', delimiter=' ')

fig1, ax1 = plt.subplots()

ax1.plot(insert2[:,0], label="$t=2$")
ax1.plot(insert3[:,0], label="$t=3$")
ax1.plot(insert4[:,0], label="$t=4$")
ax1.plot(insert5[:,0], label="$t=5$")
ax1.axis('equal')
ax1.set_xscale("log", base=2) 
ax1.grid()
ax1.autoscale()
ax1.set_title("Liczba odczytów podczas operacji wstawiania")
ax1.set_ylabel("Liczba odczytów")
ax1.set_xlabel("Liczba rekordów w indeksie")
ax1.legend()

fig1.savefig('insert.png', dpi=300)       

fig4, ax4 = plt.subplots()

ax4.plot(insert2[:,1], label="$t=2$")
ax4.plot(insert3[:,1], label="$t=3$")
ax4.plot(insert4[:,1], label="$t=4$")
ax4.plot(insert5[:,1], label="$t=5$")
ax4.axis('equal')
ax4.set_xscale("log", base=2) 
ax4.grid()
ax4.autoscale()
ax4.set_title("Liczba zapisów podczas operacji wstawiania")
ax4.set_ylabel("Liczba zapisów")
ax4.set_xlabel("Liczba rekordów w indeksie")
ax4.legend()

fig4.savefig('insert-writes.png', dpi=300)     

# ------------- search -------------------

search2 = np.genfromtxt('search-result-84.txt', delimiter=' ')
search3 = np.genfromtxt('search-result-126.txt', delimiter=' ')
search4 = np.genfromtxt('search-result-168.txt', delimiter=' ')
search5 = np.genfromtxt('search-result-210.txt', delimiter=' ')

fig2, ax2 = plt.subplots()

ax2.plot(search2[:,0], label="$t=2$")
ax2.plot(search3[:,0], label="$t=3$")
ax2.plot(search4[:,0], label="$t=4$")
ax2.plot(search5[:,0], label="$t=5$")
#ax2.axis('equal')
#ax2.set_xscale("log", base=2) 
ax2.grid()
ax2.autoscale()
ax2.set_title("Liczba odczytów podczas operacji szukania")
ax2.set_ylabel("Liczba odczytów")
ax2.set_xlabel("Numer rekordu")
ax2.legend()

fig2.savefig('search.png', dpi=300)   

n = 10000
d = 5
m = d * 2 - 1
xs = [x for x in range(n)]
pess = [math.log((n+1)/2, d) for x in xs]
#opti = [math.log(n+1,m) - 1 for x in xs]

fig3, ax3 = plt.subplots()

ax3.plot(pess, label="Pesymistyczna liczba odczytów")
#ax3.plot(opti, label="Optymistyczna liczba odczytów")
ax3.plot(search5[:,0], label="$t=5$")
#ax3.axis('equal')
#ax3.set_xscale("log", base=2) 
ax3.grid()
ax3.autoscale()
ax3.set_title("Liczba odczytów podczas operacji szukania")
ax3.set_ylabel("Liczba odczytów")
ax3.set_xlabel("Numer rekordu")
ax3.legend()

fig3.savefig('search-theory.png', dpi=300)   

# ------------- delete -------------------

delete2 = np.genfromtxt('delete-result-84.txt', delimiter=' ')
delete3 = np.genfromtxt('delete-result-126.txt', delimiter=' ')
delete4 = np.genfromtxt('delete-result-168.txt', delimiter=' ')
delete5 = np.genfromtxt('delete-result-210.txt', delimiter=' ')

fig5, ax5 = plt.subplots()

ax5.plot(delete2[:,0], label="$t=2$")
ax5.plot(delete3[:,0], label="$t=3$")
ax5.plot(delete4[:,0], label="$t=4$")
ax5.plot(delete5[:,0], label="$t=5$")
ax5.grid()
ax5.autoscale()
ax5.set_title("Liczba odczytów podczas operacji usuwania")
ax5.set_ylabel("Liczba odczytów")
ax5.set_xlabel("Numer rekordu w indeksie")
ax5.legend()

fig5.savefig('delete.png', dpi=300)   

fig6, ax6 = plt.subplots()

ax6.plot(delete2[:,1], label="$t=2$")
ax6.plot(delete3[:,1], label="$t=3$")
ax6.plot(delete4[:,1], label="$t=4$")
ax6.plot(delete5[:,1], label="$t=5$")
ax6.grid()
ax6.autoscale()
ax6.set_title("Liczba zapisów podczas operacji usuwania")
ax6.set_ylabel("Liczba zapisów")
ax6.set_xlabel("Numer rekordu w indeksie")
ax6.legend()

fig6.savefig('delete-writes.png', dpi=300)  

fig6, ax6 = plt.subplots()

size2 = np.genfromtxt('size-result-84.txt', delimiter=' ')
size3 = np.genfromtxt('size-result-126.txt', delimiter=' ')
size4 = np.genfromtxt('size-result-168.txt', delimiter=' ')
size5 = np.genfromtxt('size-result-210.txt', delimiter=' ')

fig7, ax7 = plt.subplots()

ax7.plot(size2/1024, label="$t=2$")
ax7.plot(size3/1024, label="$t=3$")
ax7.plot(size4/1024, label="$t=4$")
ax7.plot(size5/1024, label="$t=5$")
ax7.grid()
ax7.autoscale()
ax7.set_title("Rozmiar indeksu w funkcji $n$")
ax7.set_ylabel("Rozmiar indeksu $[KiB]$")
ax7.set_xlabel("Liczba rekordów w indeksie")
ax7.legend()

fig7.savefig('size.png', dpi=300)  
