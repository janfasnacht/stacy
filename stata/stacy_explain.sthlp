{smcl}
{* *! version 1.0.1 - AUTO-GENERATED}{...}
{viewerjumpto "Syntax" "stacy_explain##syntax"}{...}
{viewerjumpto "Description" "stacy_explain##description"}{...}
{viewerjumpto "Options" "stacy_explain##options"}{...}
{viewerjumpto "Returns" "stacy_explain##returns"}{...}
{viewerjumpto "Examples" "stacy_explain##examples"}{...}
{title:Title}

{phang}
{bf:stacy explain} {hline 2} Look up Stata error code details


{marker syntax}{...}
{title:Syntax}

{p 8 17 2}
{cmd:stacy explain} {it:code} 

{marker description}{...}
{title:Description}

{pstd}
{cmd:stacy explain} look up stata error code details.


{marker returns}{...}
{title:Stored results}

{pstd}
{cmd:stacy explain} stores the following in {cmd:r()}:

{synoptset 25 tabbed}{...}
{p2col 5 25 29 2: Scalars}{p_end}
{synopt:{cmd:r(code)}}Error code number{p_end}

{p2col 5 25 29 2: Macros}{p_end}
{synopt:{cmd:r(category)}}Error category{p_end}
{synopt:{cmd:r(description)}}Full error description{p_end}
{synopt:{cmd:r(name)}}Short error name{p_end}
{synopt:{cmd:r(url)}}Link to Stata documentation{p_end}


{marker examples}{...}
{title:Examples}

{pstd}Basic usage:{p_end}
{phang2}{cmd:. stacy explain}{p_end}


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
