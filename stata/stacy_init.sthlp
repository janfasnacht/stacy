{smcl}
{* *! version 0.1.0 - AUTO-GENERATED}{...}
{viewerjumpto "Syntax" "stacy_init##syntax"}{...}
{viewerjumpto "Description" "stacy_init##description"}{...}
{viewerjumpto "Options" "stacy_init##options"}{...}
{viewerjumpto "Returns" "stacy_init##returns"}{...}
{viewerjumpto "Examples" "stacy_init##examples"}{...}
{title:Title}

{phang}
{bf:stacy init} {hline 2} Initialize new stacy project


{marker syntax}{...}
{title:Syntax}

{p 8 17 2}
{cmd:stacy init} {it:path} [{cmd:,} {it:options}]

{synoptset 20 tabbed}{...}
{synopthdr}
{synoptline}
{syntab:Main}
{synopt:{opt:force}}Overwrite existing files{p_end}
{synopt:{opt:name(string)}}Project name{p_end}
{synoptline}


{marker description}{...}
{title:Description}

{pstd}
{cmd:stacy init} initialize new stacy project.


{marker options}{...}
{title:Options}

{phang}
{opt force} overwrite existing files.

{phang}
{opt name} project name.

{phang}
{opt yes} skip interactive prompts (always set in stata).


{marker returns}{...}
{title:Stored results}

{pstd}
{cmd:stacy init} stores the following in {cmd:r()}:

{synoptset 25 tabbed}{...}
{p2col 5 25 29 2: Scalars}{p_end}
{synopt:{cmd:r(created_count)}}Number of files/directories created{p_end}
{synopt:{cmd:r(package_count)}}Number of packages specified{p_end}

{p2col 5 25 29 2: Macros}{p_end}
{synopt:{cmd:r(path)}}Path where project was created{p_end}
{synopt:{cmd:r(status)}}'success' or 'error'{p_end}


{marker examples}{...}
{title:Examples}

{pstd}Basic usage:{p_end}
{phang2}{cmd:. stacy init}{p_end}


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
