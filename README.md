# libDQG ![Static Badge](https://img.shields.io/badge/Rust-gray?logo=rust&logoColor=orange) ![Static Badge](https://img.shields.io/badge/wgpu-teal?logo=wgpu) ![Static Badge](https://img.shields.io/badge/libDQG%20%3E%20dx9gf%20-%20(%E3%83%BB%CF%89%E3%83%BB)%20-orange)




Another take on my previous game framework DX9GF (A DirectX 9 Game framework).

## Building off of the good parts of DX9GF

Learning from past design flaws of DX9GF, I rethought of how to design a game framework

### Using modern tools
My biggest mistake with my previous game framework was using `DirectX 9`, an outdated, legacy system that causes many frustrations with UB and lack of QoL features. As well as `Win32`, a dogwater window library with insanely confusing design choices.

I switched to `WGPU` and `Winit` as well as `Rust` to ensure that it could be portable and crossplatform safe. Although I don't mind using `C++` with something like `SDL` + `OpenGL`, I felt like using something more robust and less worrisome would take my mind off of the dangerous nature of C++'s footguns. Sure, the Rust compiler can be pretty annoying to work with at times, but I can be at ease when it does compile.

### Object Orient Programming? More like Overly Obfuscated Programming

One of my biggest fails with my game object system (even though it's not *really* part of the core framework) was that it's designed around `inheritance` and `polymorphism`. Things like transformations were very expensive when there are even tens of entities moving around at once + hierarchies. It got really bad when the bullet hell aspect of my demo game was implemented.

So it's time to go `Data Driven Programming`, or `Entity Component System` altogether to maximize performance. Though I want to make it painless for programmers to pick up and play around with the framework since this way of thinking isn't the most intuitive for humans.

### Going 3D?
I think I did the 2D aspect of my previous game framework pretty good, the way it's designed makes it easy to pick up and understand, unlike some game frameworks i've worked with :Đ

So to push myself a bit further, I wanted to do models and meshes for this one, and pressure myself to make something actually cool with the barebone tools I made myself.

Is it going to be good? Probably not, graphics programming is a grueling task and not for the faint of heart! But it's humbling to code everything yourself and keep your binaries as tiny as possible than using a commercial game engine.

## The heck does libDQG mean?
I got some inspiration from `libGDX` naming scheme and stole it for myself :Đ

`libDQG` just means (lib)rary of (D)o(Q)in's (G)ames. As it's not meant to be a serious game framework/library, mainly for me to have full control over my game engine and licensing