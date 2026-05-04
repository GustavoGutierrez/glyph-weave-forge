# A Short Note on Heat Diffusion and Spectral Decay

## Abstract

This short mathematical article studies a one-dimensional heat equation on a
bounded interval. The goal is to provide a compact Markdown fixture with inline
and display formulas that can be used to validate future math rendering support
in GlyphWeaveForge.

## 1. Model Problem

Let $u(x,t)$ denote the temperature at position $x \in [0,L]$ and time $t > 0$.
The classical heat equation is

$$
\frac{\partial u}{\partial t} = \alpha \frac{\partial^2 u}{\partial x^2}
$$

where $\alpha > 0$ is the thermal diffusivity. We impose homogeneous boundary
conditions $u(0,t)=u(L,t)=0$ and an initial profile $u(x,0)=f(x)$.

## 2. Separation of Variables

Assume a product solution $u(x,t)=X(x)T(t)$. Substitution gives

$$
\frac{1}{\alpha T}\frac{dT}{dt} = \frac{1}{X}\frac{d^2X}{dx^2} = -\lambda
$$

The spatial eigenvalue problem is

$$
X'' + \lambda X = 0, \qquad X(0)=X(L)=0
$$

with eigenvalues and eigenfunctions

$$
\lambda_n = \left(\frac{n\pi}{L}\right)^2, \qquad X_n(x)=\sin\left(\frac{n\pi x}{L}\right).
$$

## 3. Fourier Series Solution

The solution can be written as the convergent series

$$
u(x,t) = \sum_{n=1}^{\infty} b_n \sin\left(\frac{n\pi x}{L}\right)
e^{-\alpha (n\pi/L)^2 t}
$$

where the coefficients are determined by the initial data:

$$
b_n = \frac{2}{L}\int_0^L f(x)\sin\left(\frac{n\pi x}{L}\right)\,dx.
$$

For example, if $f(x)=x(L-x)$, then each coefficient $b_n$ controls how much of
the $n$-th mode appears in the initial state.

## 4. Energy Estimate

Define the energy functional

$$
E(t)=\frac{1}{2}\int_0^L u(x,t)^2\,dx.
$$

Differentiating formally and integrating by parts yields

$$
\frac{dE}{dt} = -\alpha \int_0^L \left(\frac{\partial u}{\partial x}\right)^2 dx \leq 0.
$$

Thus the energy is non-increasing. This inequality expresses the smoothing
effect of diffusion: high-frequency oscillations decay faster because the factor
$e^{-\alpha (n\pi/L)^2t}$ becomes smaller as $n$ grows.

## 5. Numerical Stability Remark

A forward Euler discretization with grid spacing $\Delta x$ and time step
$\Delta t$ is stable under the classical condition

$$
0 < r = \frac{\alpha \Delta t}{(\Delta x)^2} \leq \frac{1}{2}.
$$

This Courant-type restriction ensures that the discrete update remains bounded.

## Conclusion

The heat equation demonstrates a central principle of parabolic PDEs: smooth
solutions emerge because spectral modes decay exponentially. This document uses
inline formulas like $\lambda_n=(n\pi/L)^2$ and display equations to exercise the
Markdown-to-PDF math pipeline.
