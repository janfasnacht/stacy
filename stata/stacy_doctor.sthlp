{smcl}
{* *! version 1.0.1 - AUTO-GENERATED}{...}
{viewerjumpto "Syntax" "stacy_doctor##syntax"}{...}
{viewerjumpto "Description" "stacy_doctor##description"}{...}
{viewerjumpto "Options" "stacy_doctor##options"}{...}
{viewerjumpto "Returns" "stacy_doctor##returns"}{...}
{viewerjumpto "Examples" "stacy_doctor##examples"}{...}
{title:Title}

{phang}
{bf:stacy doctor} {hline 2} Run system diagnostics


{marker syntax}{...}
{title:Syntax}

{p 8 17 2}
{cmd:stacy doctor} 

{marker description}{...}
{title:Description}

{pstd}
{cmd:stacy doctor} run system diagnostics.


{marker returns}{...}
{title:Stored results}

{pstd}
{cmd:stacy doctor} stores the following in {cmd:r()}:

{synoptset 25 tabbed}{...}
{p2col 5 25 29 2: Scalars}{p_end}
{synopt:{cmd:r(check_count)}}Total number of checks{p_end}
{synopt:{cmd:r(failed)}}Number of failed checks{p_end}
{synopt:{cmd:r(passed)}}Number of checks passed{p_end}
{synopt:{cmd:r(ready)}}System is ready to use (1=yes, 0=no){p_end}
{synopt:{cmd:r(warnings)}}Number of warnings{p_end}


{marker examples}{...}
{title:Examples}

{pstd}Basic usage:{p_end}
{phang2}{cmd:. stacy doctor}{p_end}


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
