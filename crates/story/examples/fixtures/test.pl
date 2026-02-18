use strict;
use warnings;
use POSIX qw(strftime);
use Scalar::Util qw(blessed looks_like_number);
use Carp qw(croak confess);

use constant VERSION => '1.0.0';
use constant LOG_LEVELS => [qw(debug info warn error)];

# ── Package / OOP ────────────────────────────────────────────────────────────

package HelloWorld;

my $instance_count = 0;

sub new {
    my ($class, %args) = @_;
    my $name    = $args{name}    // 'World';
    my $options = $args{options} // {};
    $instance_count++;
    return bless {
        name       => $name,
        options    => $options,
        created_at => time(),
    }, $class;
}

sub instance_count { $instance_count }

sub name {
    my ($self, $value) = @_;
    $self->{name} = $value if defined $value;
    return $self->{name};
}

sub greet {
    my ($self, @names) = @_;
    my @messages;
    for my $name (@names) {
        next unless length $name;
        push @messages, "Hello, $name!";
        print "Hello, $name!\n";
    }
    return wantarray ? @messages : \@messages;
}

sub configure {
    my ($self, %opts) = @_;
    $self->{options}{$_} = $opts{$_} for keys %opts;
    return $self;
}

sub process_names {
    my ($self, $names_ref) = @_;
    croak 'Expected an array ref' unless ref $names_ref eq 'ARRAY';
    return [
        sort
        map  { uc $_ }
        grep { /\S/ } @{$names_ref}
    ];
}

sub generate_report {
    my $self    = shift;
    my $elapsed = time() - $self->{created_at};
    my $stamp   = strftime('%Y-%m-%d %H:%M:%S', localtime $self->{created_at});

    return <<~REPORT;
        HelloWorld Report
        =================
        Name:    $self->{name}
        Created: $stamp
        Elapsed: ${elapsed}s
        Version: @{[ VERSION ]}
        REPORT
}

sub DESTROY { $instance_count-- if $instance_count > 0 }

# ── Utilities ────────────────────────────────────────────────────────────────

package main;

my $EMAIL_RE = qr/\A[\w+\-.]+\@[a-z\d\-]+(?:\.[a-z\d\-]+)*\.[a-z]+\z/xi;

my $validate_email = sub {
    my $email = shift;
    return $email =~ $EMAIL_RE ? 1 : 0;
};

sub describe {
    my ($val) = @_;
    return 'undef'                              unless defined $val;
    return sprintf 'Ref(%s)',  ref $val         if ref $val;
    return sprintf 'Num(%s)',  $val             if looks_like_number $val;
    return sprintf 'Str(%d): "%s"', length($val), substr($val, 0, 20);
}

sub safely (&) {
    my $block = shift;
    my $result = eval { $block->() };
    if ($@) {
        warn "Caught: $@";
        return undef;
    }
    return $result;
}

# ── Main ─────────────────────────────────────────────────────────────────────

my $greeter = HelloWorld->new(name => 'Perl');
$greeter->configure(timeout => 5000, retries => 3);

# Regex substitution and transliteration
my $title = 'hello world from perl';
(my $slug = $title) =~ s/\s+/-/g;
$slug =~ tr/a-z/A-Z/;
print "Slug: $slug\n";

# References and dereferencing
my %registry = (
    greeter => $greeter,
    tags    => [qw(demo syntax highlight)],
    meta    => { lang => 'Perl', year => 1987 },
);

my @processed = @{ $greeter->process_names([qw(alice  bob)]) };
print "Processed: @processed\n";

# List context, map/grep/sort
my @numbers = 1 .. 10;
my @evens   = grep { $_ % 2 == 0 } @numbers;
my @squares = map  { $_ ** 2 } @evens;
print "Squares of evens: @squares\n";

# Error handling
my $report = safely { $greeter->generate_report() };
print $report if defined $report;

# Special variables and string repetition
local $\ = undef;
print '-' x 40, "\n";

my @items = ($greeter, 42, undef, 'hello', \@squares);
print describe($_), "\n" for @items;

printf "Instances: %d  (v%s)\n", HelloWorld->instance_count(), VERSION;
