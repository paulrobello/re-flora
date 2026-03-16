// https://www.shadertoy.com/view/4sX3Rs
/* This is free and unencumbered software released into the public domain. https://unlicense.org/

Trying to get some interesting looking lens flares, seems like it worked.
See https://www.shadertoy.com/view/lsBGDK for a more avanced, generalized solution

If you find this useful send me an email at peterekepeter at gmail dot com,
I've seen this shader pop up before in other works, but I'm curious where it ends up.

If you want to use it, feel free to do so, there is no need to credit but it is appreciated.


Used in:

 Water World - https://www.shadertoy.com/view/lslGDB

 Floating Mountains - https://www.shadertoy.com/view/XsSGDy

 Clouds and Sun With Flare - https://www.shadertoy.com/view/4sl3zl

 musk's lens flare mod - https://www.shadertoy.com/view/XdfXRX

 Land of Dreams - https://github.com/Tomius/LoD

 Where is Starman? - https://spacein3d.com/starman


Changelog:

 13/08/13:
	first published

 17/11/27
    fixed seam, thanks to Shane

 19/08/08:
    updated description and license change

 20/06/24:
    renamed to Lens Flare Example, updated description

*/

float noise(float t)
{
	return texture(iChannel0,vec2(t,.0)/iChannelResolution[0].xy).x;
}
float noise(vec2 t)
{
	return texture(iChannel0,t/iChannelResolution[0].xy).x;
}

vec3 lensflare(vec2 uv,vec2 pos)
{
	vec2 main = uv-pos;
	vec2 uvd = uv*(length(uv));

	float ang = atan(main.x,main.y);
	float dist=length(main); dist = pow(dist,.1);
	float n = noise(vec2(ang*16.0,dist*32.0));

	float f0 = 1.0/(length(uv-pos)*16.0+1.0);

	f0 = f0 + f0*(sin(noise(sin(ang*2.+pos.x)*4.0 - cos(ang*3.+pos.y))*16.)*.1 + dist*.1 + .8);

	float f1 = max(0.01-pow(length(uv+1.2*pos),1.9),.0)*7.0;

	float f2 = max(1.0/(1.0+32.0*pow(length(uvd+0.8*pos),2.0)),.0)*00.25;
	float f22 = max(1.0/(1.0+32.0*pow(length(uvd+0.85*pos),2.0)),.0)*00.23;
	float f23 = max(1.0/(1.0+32.0*pow(length(uvd+0.9*pos),2.0)),.0)*00.21;

	vec2 uvx = mix(uv,uvd,-0.5);

	float f4 = max(0.01-pow(length(uvx+0.4*pos),2.4),.0)*6.0;
	float f42 = max(0.01-pow(length(uvx+0.45*pos),2.4),.0)*5.0;
	float f43 = max(0.01-pow(length(uvx+0.5*pos),2.4),.0)*3.0;

	uvx = mix(uv,uvd,-.4);

	float f5 = max(0.01-pow(length(uvx+0.2*pos),5.5),.0)*2.0;
	float f52 = max(0.01-pow(length(uvx+0.4*pos),5.5),.0)*2.0;
	float f53 = max(0.01-pow(length(uvx+0.6*pos),5.5),.0)*2.0;

	uvx = mix(uv,uvd,-0.5);

	float f6 = max(0.01-pow(length(uvx-0.3*pos),1.6),.0)*6.0;
	float f62 = max(0.01-pow(length(uvx-0.325*pos),1.6),.0)*3.0;
	float f63 = max(0.01-pow(length(uvx-0.35*pos),1.6),.0)*5.0;

	vec3 c = vec3(.0);

	c.r+=f2+f4+f5+f6; c.g+=f22+f42+f52+f62; c.b+=f23+f43+f53+f63;
	c = c*1.3 - vec3(length(uvd)*.05);
	c+=vec3(f0);

	return c;
}

vec3 cc(vec3 color, float factor,float factor2) // color modifier
{
	float w = color.x+color.y+color.z;
	return mix(color,vec3(w)*factor,w*factor2);
}

void mainImage( out vec4 fragColor, in vec2 fragCoord )
{
	vec2 uv = fragCoord.xy / iResolution.xy - 0.5;
	uv.x *= iResolution.x/iResolution.y; //fix aspect ratio
	vec3 mouse = vec3(iMouse.xy/iResolution.xy - 0.5,iMouse.z-.5);
	mouse.x *= iResolution.x/iResolution.y; //fix aspect ratio
	if (iMouse.z<.5)
	{
		mouse.x=sin(iTime)*.5;
		mouse.y=sin(iTime*.913)*.5;
	}

	vec3 color = vec3(1.4,1.2,1.0)*lensflare(uv,mouse.xy);
	color -= noise(fragCoord.xy)*.015;
	color = cc(color,.5,.1);
	fragColor = vec4(color,1.0);
}
