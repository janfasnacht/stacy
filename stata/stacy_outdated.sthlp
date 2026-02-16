{smcl}
{* *! version 1.0.1 - AUTO-GENERATED}{...}
{viewerjumpto "Syntax" "stacy_outdated##syntax"}{...}
{viewerjumpto "Description" "stacy_outdated##description"}{...}
{viewerjumpto "Options" "stacy_outdated##options"}{...}
{viewerjumpto "Returns" "stacy_outdated##returns"}{...}
{viewerjumpto "Examples" "stacy_outdated##examples"}{...}
{title:Title}

{phang}
{bf:stacy outdated} {hline 2} Check for package updates


{marker syntax}{...}
{title:Syntax}

{p 8 17 2}
{cmd:stacy outdated} 

{marker description}{...}
{title:Description}

{pstd}
{cmd:stacy outdated} check for package updates.


{marker returns}{...}
{title:Stored results}

{pstd}
{cmd:stacy outdated} stores the following in {cmd:r()}:

{synoptset 25 tabbed}{...}
{p2col 5 25 29 2: Scalars}{p_end}
{synopt:{cmd:r(outdated_count)}}Number of outdated packages{p_end}
{synopt:{cmd:r(total_count)}}Total packages checked{p_end}

{p2col 5 25 29 2: Macros}{p_end}
{synopt:{cmd:r(outdated_currents)}}Comma-separated current versions{p_end}
{synopt:{cmd:r(outdated_latests)}}Comma-separated latest versions{p_end}
{synopt:{cmd:r(outdated_names)}}Comma-separated outdated package names{p_end}
{synopt:{cmd:r(status)}}'success' or 'error'{p_end}


{marker examples}{...}
{title:Examples}

{pstd}Basic usage:{p_end}
{phang2}{cmd:. stacy outdated}{p_end}


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
