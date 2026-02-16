{smcl}
{* *! version 1.0.1 - AUTO-GENERATED}{...}
{viewerjumpto "Syntax" "stacy_remove##syntax"}{...}
{viewerjumpto "Description" "stacy_remove##description"}{...}
{viewerjumpto "Options" "stacy_remove##options"}{...}
{viewerjumpto "Returns" "stacy_remove##returns"}{...}
{viewerjumpto "Examples" "stacy_remove##examples"}{...}
{title:Title}

{phang}
{bf:stacy remove} {hline 2} Remove packages from project


{marker syntax}{...}
{title:Syntax}

{p 8 17 2}
{cmd:stacy remove} {it:packages} 

{marker description}{...}
{title:Description}

{pstd}
{cmd:stacy remove} remove packages from project.


{marker returns}{...}
{title:Stored results}

{pstd}
{cmd:stacy remove} stores the following in {cmd:r()}:

{synoptset 25 tabbed}{...}
{p2col 5 25 29 2: Scalars}{p_end}
{synopt:{cmd:r(not_found)}}Number of packages not found{p_end}
{synopt:{cmd:r(removed)}}Number of packages removed{p_end}
{synopt:{cmd:r(total)}}Total packages processed{p_end}

{p2col 5 25 29 2: Macros}{p_end}
{synopt:{cmd:r(status)}}'success' or 'error'{p_end}


{marker examples}{...}
{title:Examples}

{pstd}Basic usage:{p_end}
{phang2}{cmd:. stacy remove}{p_end}


{marker author}{...}
{title:Author}

{pstd}
Jan Fasnacht{p_end}
{pstd}
{browse "https://github.com/janfasnacht/stacy":github.com/janfasnacht/stacy}{p_end}


{marker also_see}{...}
{title:Also see}

{psee}
{space 2}Help:  {helpb stacy}
{p_end}
